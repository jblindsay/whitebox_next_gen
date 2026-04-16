#!/usr/bin/env python3
"""Pre-generate rich HTML help files for tools that lack a bundled help page.

Outputs are written directly into help_static/ so they ship with the plugin
and are served as first-class bundled files (no runtime generation needed).

Usage (from repo root, with the wbw venv active):
    python crates/wbw_qgis/plugin/scripts/generate_static_help.py

Options:
    --force      Overwrite existing files (default: skip already-generated
                 files that are NOT the originals — i.e. skip legacy files
                 but re-generate any previously script-generated ones).
    --only TOOL  Generate only the given tool_id (useful for spot-checks).
    --dry-run    Show what would be written without writing anything.
"""
from __future__ import annotations

import argparse
import json
import os
import sys
from pathlib import Path

# ---------------------------------------------------------------------------
# Locate repo root and add plugin to sys.path
# ---------------------------------------------------------------------------
SCRIPT_DIR = Path(__file__).resolve().parent
REPO_ROOT = SCRIPT_DIR.parents[3]
PLUGIN_DIR = REPO_ROOT / "crates" / "wbw_qgis" / "plugin" / "whitebox_workflows_qgis"
DESCRIPTIONS_DIR = PLUGIN_DIR / "descriptions"
HELP_STATIC_DIR = PLUGIN_DIR / "help_static"

sys.path.insert(0, str(PLUGIN_DIR.parent))


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

def load_all_descriptions() -> dict[str, dict]:
    """Load all curated + auto-generated description JSON files.

    Returns a mapping of tool_id → description dict.
    Auto-generated baseline is loaded first; curated files override it.
    """
    merged: dict[str, dict] = {}

    auto_path = DESCRIPTIONS_DIR / "auto_generated_tier1.json"
    if auto_path.is_file():
        with open(auto_path, encoding="utf-8") as fh:
            data = json.load(fh)
        for tool_id, entry in data.items():
            merged[tool_id] = entry

    # Curated files override (sorted for determinism)
    for json_path in sorted(DESCRIPTIONS_DIR.glob("*.json")):
        if json_path.name == "auto_generated_tier1.json":
            continue
        with open(json_path, encoding="utf-8") as fh:
            data = json.load(fh)
        for tool_id, entry in data.items():
            merged[tool_id] = entry

    return merged


def _badges(is_pro: bool, stability: str) -> str:
    parts = []
    if is_pro:
        parts.append(
            '<span style="background:#c0392b;color:#fff;padding:2px 8px;'
            'border-radius:3px;font-size:0.85em;margin-right:6px;">PRO</span>'
        )
    if stability:
        colour = "#e67e22" if stability.lower() == "experimental" else "#27ae60"
        parts.append(
            f'<span style="background:{colour};color:#fff;padding:2px 8px;'
            f'border-radius:3px;font-size:0.85em;margin-right:6px;">{stability}</span>'
        )
    return f'<p>{"".join(parts)}</p>\n' if parts else ""


def _param_table(params: list[dict], defaults: dict, curated_params: dict) -> str:
    if not params:
        return ""
    rows = ""
    for p in params:
        name = p.get("name", "")
        manifest_desc = p.get("description", "")
        curated_entry = curated_params.get(name, {})
        label = curated_entry.get("label", "") or manifest_desc
        tooltip = curated_entry.get("tooltip", "")
        cell_desc = label
        if tooltip and tooltip.strip() != label.strip():
            cell_desc = (
                f"{label}<br>"
                f"<small style='color:#555'>{tooltip}</small>"
            )
        req = (
            '<span style="color:#c0392b">Required</span>'
            if p.get("required", False)
            else "Optional"
        )
        default_val = defaults.get(name, "")
        default_cell = f"<code>{default_val}</code>" if default_val != "" else "—"
        rows += (
            f"<tr>"
            f"<td style='padding:4px 8px;border-bottom:1px solid #eee'><code>{name}</code></td>"
            f"<td style='padding:4px 8px;border-bottom:1px solid #eee'>{cell_desc}</td>"
            f"<td style='padding:4px 8px;border-bottom:1px solid #eee'>{req}</td>"
            f"<td style='padding:4px 8px;border-bottom:1px solid #eee'>{default_cell}</td>"
            f"</tr>\n"
        )
    return (
        "<h2>Parameters</h2>\n"
        "<table style='border-collapse:collapse;width:100%'>"
        "<thead>"
        "<tr style='background:#f5f5f5'>"
        "<th style='text-align:left;padding:4px 8px;border-bottom:2px solid #ccc'>Name</th>"
        "<th style='text-align:left;padding:4px 8px;border-bottom:2px solid #ccc'>Description</th>"
        "<th style='text-align:left;padding:4px 8px;border-bottom:2px solid #ccc'>Required</th>"
        "<th style='text-align:left;padding:4px 8px;border-bottom:2px solid #ccc'>Default</th>"
        "</tr></thead>\n"
        f"<tbody>{rows}</tbody></table>\n"
    )


def _examples_section(tool_id: str, examples: list[dict]) -> str:
    if not examples:
        return ""
    html = "<h2>Examples</h2>\n"
    for ex in examples:
        ex_desc = ex.get("description", "")
        ex_args = ex.get("args", {})
        if ex_desc:
            html += f"<p><em>{ex_desc}</em></p>\n"
        if ex_args:
            args_str = ", ".join(f"{k}={repr(v)}" for k, v in ex_args.items())
            html += f"<pre><code>wbe.{tool_id}({args_str})</code></pre>\n"
    return html


def _tags_line(tags: list[str]) -> str:
    if not tags:
        return ""
    spans = " ".join(
        f'<span style="background:#eef;padding:1px 7px;border-radius:10px;'
        f'font-size:0.82em;margin-right:4px">{t}</span>'
        for t in tags
    )
    return f"<p>{spans}</p>\n"


def generate_html(manifest: dict, curated: dict | None) -> str:
    tool_id = manifest.get("id", "")
    summary = manifest.get("summary", "No description available.")
    params = manifest.get("params", [])
    defaults = manifest.get("defaults", {})
    examples = manifest.get("examples", [])
    tags = manifest.get("tags", [])
    stability = manifest.get("stability", "")
    license_tier = manifest.get("license_tier_name", manifest.get("license_tier", "open"))
    is_pro = str(license_tier).lower() in ("pro", "enterprise")

    curated_params: dict = {}
    curated_summary = ""
    if curated:
        curated_params = curated.get("params", {})
        curated_summary = curated.get("description", "")

    effective_summary = curated_summary or summary

    return (
        f"{_badges(is_pro, stability)}"
        f"<p>{effective_summary}</p>\n"
        f"{_tags_line(tags)}"
        f"{_param_table(params, defaults, curated_params)}"
        f"{_examples_section(tool_id, examples)}"
        "<h2>Project Links</h2>\n"
        '<div align="left">\n'
        '    <a href="https://www.whiteboxgeo.com/whitebox-workflows-for-python/">WbW Homepage</a>\n'
        '    <a href="https://www.whiteboxgeo.com/manual/wbw-user-manual/book/preface.html">User Manual</a>\n'
        '    <a href="https://www.whiteboxgeo.com/whitebox-workflows/">Learn More</a>\n'
        "</div>\n"
    )


# ---------------------------------------------------------------------------
# Sentinel: mark script-generated files so --force can overwrite them
# (legacy original files are NOT marked — they are never overwritten)
# ---------------------------------------------------------------------------
GENERATED_MARKER = "<!-- wbw-qgis:generated -->"


def _is_script_generated(path: Path) -> bool:
    try:
        return GENERATED_MARKER in path.read_text(encoding="utf-8", errors="ignore")
    except OSError:
        return False


# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------

def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--force", action="store_true",
                        help="Overwrite previously script-generated files")
    parser.add_argument("--only", metavar="TOOL_ID",
                        help="Generate only a single tool (for testing)")
    parser.add_argument("--dry-run", action="store_true",
                        help="Print actions but do not write files")
    args = parser.parse_args()

    try:
        import whitebox_workflows as wbw
    except ImportError:
        sys.exit("ERROR: whitebox_workflows not importable. Activate the wbw venv first.")

    catalog: list[dict] = json.loads(wbw.list_tool_catalog_json())
    descriptions = load_all_descriptions()

    if args.only:
        catalog = [t for t in catalog if t["id"] == args.only]
        if not catalog:
            sys.exit(f"ERROR: tool '{args.only}' not found in catalog.")

    written = skipped_legacy = skipped_existing = 0

    for item in catalog:
        tool_id = item.get("id", "")
        out_path = HELP_STATIC_DIR / f"{tool_id}.html"

        if out_path.is_file():
            if not _is_script_generated(out_path):
                # Original legacy file — never touch it
                skipped_legacy += 1
                continue
            if not args.force:
                skipped_existing += 1
                continue

        html = GENERATED_MARKER + "\n" + generate_html(item, descriptions.get(tool_id))

        if args.dry_run:
            print(f"[dry-run] would write {out_path}")
        else:
            out_path.write_text(html, encoding="utf-8")
            print(f"  wrote {tool_id}.html")
        written += 1

    print(
        f"\nDone. written={written}, "
        f"skipped legacy originals={skipped_legacy}, "
        f"skipped existing generated={skipped_existing}"
    )


if __name__ == "__main__":
    main()
