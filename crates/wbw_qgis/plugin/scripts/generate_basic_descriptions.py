#!/usr/bin/env python3
"""Generate baseline parameter descriptions for remaining Tier 1 tools.

This script builds an auto-generated JSON description pack for tools that:
1) are in Tier 1 scope,
2) do not have a legacy help HTML file, and
3) are not already covered by curated description JSON files.

Usage:
  python crates/wbw_qgis/plugin/scripts/generate_basic_descriptions.py
  python crates/wbw_qgis/plugin/scripts/generate_basic_descriptions.py --output crates/wbw_qgis/plugin/whitebox_workflows_qgis/descriptions/auto_generated_tier1.json
"""

from __future__ import annotations

import argparse
import json
import os
import re
import sys
from pathlib import Path
from typing import Any


REPO_ROOT = Path(__file__).resolve().parents[4]
PLUGIN_ROOT = REPO_ROOT / "crates/wbw_qgis/plugin"
DESCRIPTIONS_DIR = PLUGIN_ROOT / "whitebox_workflows_qgis/descriptions"
DEFAULT_OUTPUT = DESCRIPTIONS_DIR / "auto_generated_tier1.json"
DEFAULT_HELP_DIR = Path.home() / "Documents/programming/Rust/whitebox_workflows/wbw_qgis/help"

TIER1_PATTERNS = [
    "vector",
    "hydrology",
    "terrain",
    "geomorphometry",
    "classification",
    "remote_sensing",
    "flow",
    "streams",
    "network",
    "linear-referencing",
]

MAX_LABEL_CHARS = 110


def _normalize_spaces(text: str) -> str:
    return re.sub(r"\s+", " ", text).strip()


def _humanize_param_name(name: str) -> str:
    words = name.replace("-", "_").split("_")
    words = [w for w in words if w]
    if not words:
        return "Parameter"
    acronyms = {"dem": "DEM", "epsg": "EPSG", "lidar": "LiDAR", "sar": "SAR", "ndvi": "NDVI", "nir": "NIR", "swir": "SWIR"}
    out = []
    for w in words:
        lw = w.lower()
        if lw in acronyms:
            out.append(acronyms[lw])
        elif len(w) <= 2 and w.isalpha():
            out.append(w.upper())
        else:
            out.append(lw.capitalize())
    return " ".join(out)


def _humanize_bool_label(name: str) -> str:
    n = name.lower().strip()
    if n.startswith("enable_"):
        return f"Enable {_humanize_param_name(n[len('enable_'):])}"
    if n.startswith("use_"):
        return f"Use {_humanize_param_name(n[len('use_'):])}"
    if n.startswith("include_"):
        return f"Include {_humanize_param_name(n[len('include_'):])}"
    if n.startswith("keep_"):
        return f"Keep {_humanize_param_name(n[len('keep_'):])}"
    if n.startswith("only_"):
        return f"Only {_humanize_param_name(n[len('only_'):])}"
    return _humanize_param_name(name)


def _shorten_label(label: str, max_chars: int = MAX_LABEL_CHARS) -> str:
    if len(label) <= max_chars:
        return label
    trimmed = label[:max_chars].rsplit(" ", 1)[0].rstrip(".,;: ")
    if not trimmed:
        return label[:max_chars].rstrip() + "..."
    return trimmed + "..."


def _is_bool_like(name: str, desc: str) -> bool:
    n = name.lower()
    d = desc.lower()
    return (
        n.startswith(("is_", "has_", "use_", "enable_", "include_", "only_", "keep_"))
        or "if true" in d
        or "enable" in d
        or "disable" in d
        or "boolean" in d
    )


def _first_sentence(text: str) -> str:
    text = _normalize_spaces(text)
    if not text:
        return ""
    parts = re.split(r"(?<=[.!?])\s+", text, maxsplit=1)
    return parts[0]


def _render_default(value: Any) -> str:
    if value is None:
        return ""
    if isinstance(value, str):
        v = value.strip()
        if not v:
            return ""
        return v
    return str(value)


def _build_label(param: dict[str, Any], defaults: dict[str, Any]) -> str:
    name = str(param.get("name", "")).strip()
    if not name:
        return "Parameter"

    desc = _normalize_spaces(str(param.get("description", "") or ""))
    required = bool(param.get("required", False))

    default_value = param.get("default")
    if default_value is None and name in defaults:
        default_value = defaults.get(name)

    pretty_name = _humanize_param_name(name)
    sentence = _first_sentence(desc)

    if _is_bool_like(name, desc):
        label = _humanize_bool_label(name)
        if not required:
            label = f"{label} [optional]"
        return _normalize_spaces(label)

    if sentence:
        label = sentence
        if label and label[0].islower():
            label = label[0].upper() + label[1:]
    else:
        label = f"{pretty_name}."

    if not label.endswith("."):
        label = f"{label}."

    default_text = _render_default(default_value)
    if default_text and "default" not in label.lower():
        label = f"{label} (default {default_text})."

    if not required:
        label = f"{label} [optional]"

    return _shorten_label(_normalize_spaces(label))


def _build_tooltip(param: dict[str, Any]) -> str | None:
    name = str(param.get("name", "")).strip()
    desc = _normalize_spaces(str(param.get("description", "") or ""))
    if not desc:
        return None

    name_l = name.lower()
    enum_like_name = any(tok in name_l for tok in ("mode", "method", "strategy", "profile", "type"))
    enum_like_desc = (
        "|" in desc
        or "options" in desc.lower()
        or (":" in desc and "," in desc)
    )

    if _is_bool_like(name, desc):
        return desc

    if enum_like_name or enum_like_desc:
        return desc

    if len(desc) > 95:
        return desc

    return None


def _load_existing_curated_ids(descriptions_dir: Path) -> set[str]:
    curated_ids: set[str] = set()
    for fp in descriptions_dir.glob("*.json"):
        if fp.name == "auto_generated_tier1.json":
            continue
        try:
            data = json.loads(fp.read_text(encoding="utf-8"))
            if isinstance(data, dict):
                curated_ids.update(data.keys())
        except Exception:
            continue
    return curated_ids


def _in_tier1(tool: dict[str, Any]) -> bool:
    cat = str(tool.get("category", "")).lower()
    tags = [str(t).lower() for t in tool.get("tags", [])]
    return any(p in cat or any(p in t for t in tags) for p in TIER1_PATTERNS)


def main() -> int:
    parser = argparse.ArgumentParser(description="Generate baseline Tier 1 descriptions.")
    parser.add_argument("--output", type=Path, default=DEFAULT_OUTPUT)
    parser.add_argument("--help-dir", type=Path, default=DEFAULT_HELP_DIR)
    args = parser.parse_args()

    sys.path.insert(0, str(PLUGIN_ROOT))
    from whitebox_workflows_qgis.discovery import discover_tool_catalog  # pylint: disable=import-error

    catalog = discover_tool_catalog(include_pro=True, tier="open")

    available_help = set()
    if args.help_dir.exists():
        available_help = {
            f[:-5] for f in os.listdir(args.help_dir) if f.endswith(".html")
        }

    curated_ids = _load_existing_curated_ids(DESCRIPTIONS_DIR)

    generated: dict[str, Any] = {}
    count_tier1 = 0
    for tool in catalog:
        if not _in_tier1(tool):
            continue
        count_tier1 += 1

        tool_id = str(tool.get("id", "")).strip()
        if not tool_id:
            continue
        if tool_id in available_help:
            continue
        if tool_id in curated_ids:
            continue

        params = tool.get("params", [])
        if not params:
            continue

        defaults = tool.get("defaults", {})
        entry = {
            "description": _first_sentence(_normalize_spaces(str(tool.get("description", "") or ""))),
            "parameters": {},
        }

        for p in params:
            name = str(p.get("name", "")).strip()
            if not name:
                continue
            label = _build_label(p, defaults)
            tooltip = _build_tooltip(p)
            param_entry: dict[str, Any] = {"label": label}
            if tooltip:
                param_entry["tooltip"] = tooltip
            entry["parameters"][name] = param_entry

        if entry["parameters"]:
            generated[tool_id] = entry

    args.output.parent.mkdir(parents=True, exist_ok=True)
    with open(args.output, "w", encoding="utf-8") as f:
        json.dump(dict(sorted(generated.items())), f, indent=2, ensure_ascii=True)
        f.write("\n")

    print(f"Tier1 tools scanned: {count_tier1}")
    print(f"Legacy-help tools skipped: {len([1 for t in catalog if _in_tier1(t) and str(t.get('id','')) in available_help])}")
    print(f"Already-curated tools skipped: {len([1 for t in catalog if _in_tier1(t) and str(t.get('id','')) in curated_ids])}")
    print(f"Auto-generated tools written: {len(generated)}")
    print(f"Output: {args.output}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
