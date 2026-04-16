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
import html
import json
import os
import re
import sys
from pathlib import Path
from typing import Any

# ---------------------------------------------------------------------------
# Locate repo root and add plugin to sys.path
# ---------------------------------------------------------------------------
SCRIPT_DIR = Path(__file__).resolve().parent
REPO_ROOT = SCRIPT_DIR.parents[3]
PLUGIN_DIR = REPO_ROOT / "crates" / "wbw_qgis" / "plugin" / "whitebox_workflows_qgis"
DESCRIPTIONS_DIR = PLUGIN_DIR / "descriptions"
HELP_STATIC_DIR = PLUGIN_DIR / "help_static"
WBTOOLS_PRO_README_DEFAULT = REPO_ROOT.parent / "wbtools_pro" / "README.md"

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


def _extract_bullets(lines: list[str]) -> list[str]:
    out: list[str] = []
    for ln in lines:
        s = ln.strip()
        if not s:
            continue
        if s.startswith("- "):
            out.append(s[2:].strip())
        else:
            out.append(s)
    return out


def _clean_scalar(lines: list[str]) -> str:
    text = " ".join(lines).strip()
    if text.startswith("- "):
        text = text[2:].strip()
    return text


def _normalize_label_line(line: str) -> str:
    s = line.strip().strip("*").strip()
    s = re.sub(r"\s+", " ", s)
    return s.lower()


def _extract_field_block(section: str, label: str, all_labels: list[str]) -> list[str]:
    lines = section.splitlines()
    start = -1
    label_lc = label.lower()
    inline_first: str = ""
    for i, ln in enumerate(lines):
        norm = _normalize_label_line(ln)
        if norm == label_lc:
            start = i + 1
            break
        if norm.startswith(label_lc):
            # Handles inline styles, e.g. "**Primary buyer:** text".
            inline_first = norm[len(label_lc):].strip()
            inline_first = inline_first.lstrip(":").strip()
            start = i + 1
            break
    if start < 0:
        return []

    out: list[str] = [inline_first] if inline_first else []
    label_set = {x.lower() for x in all_labels}
    for ln in lines[start:]:
        stripped = ln.strip()
        norm = _normalize_label_line(stripped)
        if norm in label_set:
            break
        if stripped.startswith("##"):
            break
        if norm in {
            "inputs:",
            "outputs:",
            "python example:",
            "**inputs:**",
            "**outputs:**",
            "**python example:**",
        }:
            break
        out.append(ln)
    return out


def _clean_heading_title(raw: str) -> str:
    title = raw.strip().lstrip("#").strip()
    # Strip leading section numbering like "1.4 " or "6.8 ".
    title = re.sub(r"^\d+(?:\.\d+)*\s+", "", title)
    return title


def _find_pro_readme_path() -> Path | None:
    env_path = os.environ.get("WBTOOLS_PRO_README", "").strip()
    if env_path:
        p = Path(env_path)
        if p.is_file():
            return p
    if WBTOOLS_PRO_README_DEFAULT.is_file():
        return WBTOOLS_PRO_README_DEFAULT
    return None


def _extract_markdown_table(section: str, start_label: str, end_label: str | None) -> list[str]:
    lines = section.splitlines()
    start = -1
    for i, ln in enumerate(lines):
        norm = _normalize_label_line(ln)
        if norm == start_label.lower() or norm == f"**{start_label.lower()}**":
            start = i + 1
            break
    if start < 0:
        return []

    out: list[str] = []
    for ln in lines[start:]:
        stripped = ln.rstrip()
        norm = _normalize_label_line(stripped)
        if end_label and (norm == end_label.lower() or norm == f"**{end_label.lower()}**"):
            break
        if stripped.startswith("### "):
            break
        if stripped.strip().startswith("```"):
            break
        if stripped.strip().startswith("|"):
            out.append(stripped)
        elif out and not stripped.strip():
            break
    return out


def _extract_python_example(section: str) -> str:
    m = re.search(r"(?:\*\*\s*)?Python example:(?:\s*\*\*)?\s*\n```python\n(.*?)\n```", section, flags=re.DOTALL)
    if not m:
        return ""
    return m.group(1).strip()


def _markdown_table_to_html(table_lines: list[str], heading: str) -> str:
    if len(table_lines) < 2:
        return ""

    def parse_row(line: str) -> list[str]:
        # Preserve escaped pipes (\|) within cell content.
        safe = line.replace("\\|", "__PIPE__")
        cells = [c.strip().replace("__PIPE__", "|") for c in safe.strip().strip("|").split("|")]
        return [html.escape(c) for c in cells]

    header = parse_row(table_lines[0])
    body_rows = [parse_row(ln) for ln in table_lines[2:] if ln.strip()]

    thead = "".join(
        f"<th style='text-align:left;padding:4px 8px;border-bottom:2px solid #ccc'>{h}</th>"
        for h in header
    )
    tbody = ""
    for row in body_rows:
        tbody += "<tr>" + "".join(
            f"<td style='padding:4px 8px;border-bottom:1px solid #eee'>{c}</td>" for c in row
        ) + "</tr>\n"

    return (
        f"<h2>{heading}</h2>\n"
        "<table style='border-collapse:collapse;width:100%'>"
        f"<thead><tr style='background:#f5f5f5'>{thead}</tr></thead>\n"
        f"<tbody>{tbody}</tbody></table>\n"
    )


def load_wbtools_pro_marketing() -> dict[str, dict[str, Any]]:
    """Load per-tool narrative fields from wbtools_pro/README.md.

    Returns mapping: tool_id -> extracted fields.
    """
    readme_path = _find_pro_readme_path()
    if readme_path is None:
        return {}

    text = readme_path.read_text(encoding="utf-8")
    lines = text.splitlines()

    sections: list[tuple[str, str]] = []
    current_heading = ""
    current_lines: list[str] = []
    for line in lines:
        if line.startswith("### "):
            if current_heading and current_lines:
                sections.append((current_heading, "\n".join(current_lines)))
            current_heading = line
            current_lines = []
        elif current_heading:
            current_lines.append(line)
    if current_heading and current_lines:
        sections.append((current_heading, "\n".join(current_lines)))

    labels = [
        "What it does:",
        "How it works (calculation method):",
        "Who it is for:",
        "Primary buyer:",
        "Business question it answers:",
        "Why it wins vs alternatives:",
        "Typical buying trigger:",
        "Typical presets:",
    ]

    out: dict[str, dict[str, Any]] = {}
    for heading, section_text in sections:
        m = re.search(r"^Tool ID:\s*([a-z0-9_]+)\s*$", section_text, flags=re.MULTILINE)
        if not m:
            continue
        tool_id = m.group(1).strip()

        out[tool_id] = {
            "title": _clean_heading_title(heading),
            "what_it_does": _extract_bullets(_extract_field_block(section_text, "What it does:", labels)),
            "how_it_works": _extract_bullets(_extract_field_block(section_text, "How it works (calculation method):", labels)),
            "who_for": _extract_bullets(_extract_field_block(section_text, "Who it is for:", labels)),
            "primary_buyer": _clean_scalar(_extract_field_block(section_text, "Primary buyer:", labels)),
            "business_question": _clean_scalar(_extract_field_block(section_text, "Business question it answers:", labels)),
            "why_wins": _extract_bullets(_extract_field_block(section_text, "Why it wins vs alternatives:", labels)),
            "buying_trigger": _clean_scalar(_extract_field_block(section_text, "Typical buying trigger:", labels)),
            "presets": _extract_bullets(_extract_field_block(section_text, "Typical presets:", labels)),
            "inputs_table": _extract_markdown_table(section_text, "Inputs:", "Outputs:"),
            "outputs_table": _extract_markdown_table(section_text, "Outputs:", "Python example:"),
            "python_example": _extract_python_example(section_text),
        }

    return out


def _bullet_html(items: list[str]) -> str:
    if not items:
        return ""
    return "<ul>" + "".join(f"<li>{item}</li>" for item in items if item) + "</ul>"


def _pro_narrative_section(tool_id: str, pro_marketing: dict[str, dict]) -> str:
    info = pro_marketing.get(tool_id)
    if not info:
        return ""

    blocks: list[str] = ["<h2>Workflow Narrative</h2>"]

    title = str(info.get("title", "")).strip()
    if title:
        blocks.append(f"<p><strong>{title}</strong></p>")

    if info.get("business_question"):
        blocks.append("<h3>Problem It Solves</h3>")
        blocks.append(f"<p>{info['business_question']}</p>")

    if info.get("who_for"):
        blocks.append("<h3>Who It Is For</h3>")
        blocks.append(_bullet_html(info["who_for"]))

    if info.get("primary_buyer"):
        blocks.append("<h3>Primary Buyer</h3>")
        blocks.append(f"<p>{info['primary_buyer']}</p>")

    if info.get("what_it_does"):
        blocks.append("<h3>What It Does</h3>")
        blocks.append(_bullet_html(info["what_it_does"]))

    if info.get("how_it_works"):
        blocks.append("<h3>How It Works</h3>")
        blocks.append(_bullet_html(info["how_it_works"]))

    if info.get("why_wins"):
        blocks.append("<h3>Why It Wins</h3>")
        blocks.append(_bullet_html(info["why_wins"]))

    if info.get("buying_trigger"):
        blocks.append("<h3>Typical Buying Trigger</h3>")
        blocks.append(f"<p>{info['buying_trigger']}</p>")

    if info.get("presets"):
        blocks.append("<h3>Typical Presets</h3>")
        blocks.append(_bullet_html(info["presets"]))

    return "\n".join(blocks) + "\n"


def _pro_detail_sections(tool_id: str, pro_marketing: dict[str, dict]) -> str:
    info = pro_marketing.get(tool_id)
    if not info:
        return ""

    out = ""
    out += _markdown_table_to_html(info.get("inputs_table", []), "Inputs")
    out += _markdown_table_to_html(info.get("outputs_table", []), "Outputs")

    py = str(info.get("python_example", "")).strip()
    if py:
        out += "<h2>Python Example</h2>\n"
        out += f"<pre><code>{html.escape(py)}</code></pre>\n"

    return out


def generate_html(manifest: dict, curated: dict | None, pro_marketing: dict[str, dict]) -> str:
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
    pro_narrative = _pro_narrative_section(tool_id, pro_marketing) if is_pro else ""
    pro_details = _pro_detail_sections(tool_id, pro_marketing) if is_pro else ""

    return (
        f"{_badges(is_pro, stability)}"
        f"<p>{effective_summary}</p>\n"
        f"{_tags_line(tags)}"
        f"{pro_narrative}"
        f"{pro_details}"
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
    parser.add_argument("--pro-only", action="store_true",
                        help="Generate only Pro-tier tools")
    parser.add_argument(
        "--overwrite-existing-pro",
        action="store_true",
        help="When used with --pro-only, allow overwriting existing bundled Pro help files",
    )
    parser.add_argument(
        "--workflow-pro-readme",
        action="store_true",
        help="Generate help files directly from wbtools_pro README sections for all 42 workflow tools",
    )
    args = parser.parse_args()

    try:
        import whitebox_workflows as wbw
    except ImportError:
        sys.exit("ERROR: whitebox_workflows not importable. Activate the wbw venv first.")

    catalog: list[dict] = json.loads(wbw.list_tool_catalog_json())
    descriptions = load_all_descriptions()
    pro_marketing = load_wbtools_pro_marketing()

    if args.only:
        catalog = [t for t in catalog if t["id"] == args.only]
        if not catalog:
            sys.exit(f"ERROR: tool '{args.only}' not found in catalog.")

    if args.pro_only:
        catalog = [
            t
            for t in catalog
            if str(t.get("license_tier_name", t.get("license_tier", "open"))).lower()
            in ("pro", "enterprise")
        ]

    if args.workflow_pro_readme:
        workflow_ids = set(pro_marketing.keys())
        catalog = [t for t in catalog if t.get("id", "") in workflow_ids]

        # Add synthetic entries for workflow tools not present in runtime catalog.
        present = {t.get("id", "") for t in catalog}
        missing = sorted(workflow_ids - present)
        for tool_id in missing:
            info = pro_marketing.get(tool_id, {})
            catalog.append(
                {
                    "id": tool_id,
                    "display_name": info.get("title", tool_id.replace("_", " ").title()),
                    "summary": "Workflow-grade Pro analysis with audit-ready outputs.",
                    "params": [],
                    "defaults": {},
                    "examples": [],
                    "tags": ["workflow", "pro"],
                    "stability": "Production",
                    "license_tier": "pro",
                    "license_tier_name": "pro",
                }
            )

    written = skipped_legacy = skipped_existing = 0

    for item in catalog:
        tool_id = item.get("id", "")
        out_path = HELP_STATIC_DIR / f"{tool_id}.html"
        is_pro = str(item.get("license_tier_name", item.get("license_tier", "open"))).lower() in (
            "pro",
            "enterprise",
        )

        if out_path.is_file():
            if not _is_script_generated(out_path):
                # Original legacy file — never touch it
                if not (
                    args.pro_only
                    and args.overwrite_existing_pro
                    and is_pro
                ):
                    skipped_legacy += 1
                    continue
            if not args.force:
                skipped_existing += 1
                continue

        html = GENERATED_MARKER + "\n" + generate_html(item, descriptions.get(tool_id), pro_marketing)

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
