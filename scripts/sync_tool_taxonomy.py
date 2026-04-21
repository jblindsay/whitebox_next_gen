#!/usr/bin/env python3
"""Sync curated tool taxonomy into Python API category mapping.

Source of truth:
- crates/wbw_python/tool_taxonomy.toml

Primary targets:
- crates/wbw_python/src/wb_environment.rs
- crates/wbw_python/tool_taxonomy.resolved.json
- optional QGIS taxonomy export

Validation:
- duplicate/conflicting taxonomy assignments
- stub coverage for curated subcategories
- generated mapping drift via --check
"""

from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path
from typing import Any

try:
    import tomllib
except ModuleNotFoundError:  # pragma: no cover
    import tomli as tomllib  # type: ignore


ROOT = Path(__file__).resolve().parents[1]
TAXONOMY_PATH = ROOT / "crates/wbw_python/tool_taxonomy.toml"
WB_ENV_PATH = ROOT / "crates/wbw_python/src/wb_environment.rs"
STUB_PATH = ROOT / "crates/wbw_python/whitebox_workflows/whitebox_workflows.pyi"
R_TAXONOMY_PATH = ROOT / "crates/wbw_r/r-package/whiteboxworkflows/inst/extdata/tool_taxonomy.resolved.json"
QGIS_TAXONOMY_PATH = ROOT / "crates/wbw_qgis/plugin/whitebox_workflows_qgis/tool_taxonomy.resolved.json"

RS_BEGIN = "// BEGIN AUTO-TAXONOMY-MAP"
RS_END = "// END AUTO-TAXONOMY-MAP"
RS_SUBCATS_BEGIN = "// BEGIN AUTO-TAXONOMY-SUBCATEGORIES"
RS_SUBCATS_END = "// END AUTO-TAXONOMY-SUBCATEGORIES"
STUB_AUTO_BEGIN = "# BEGIN AUTO-TAXONOMY-SUBCATEGORY-STUBS"
STUB_AUTO_END = "# END AUTO-TAXONOMY-SUBCATEGORY-STUBS"


def slug_to_camel(slug: str) -> str:
    return "".join(part.capitalize() for part in slug.split("_"))


def stub_class_name(category: str, subcategory: str) -> str:
    return f"Wb{slug_to_camel(category)}{slug_to_camel(subcategory)}Subcategory"


def taxonomy_subcategories_by_category(
    mappings: list[dict[str, object]],
) -> list[tuple[str, list[str]]]:
    grouped: dict[str, list[str]] = {}
    ordered_categories: list[str] = []
    for entry in mappings:
        category = str(entry["category"])
        subcategory = str(entry["subcategory"])
        if category not in grouped:
            grouped[category] = []
            ordered_categories.append(category)
        if subcategory not in grouped[category]:
            grouped[category].append(subcategory)
    return [(category, grouped[category]) for category in ordered_categories]


def load_taxonomy(path: Path) -> list[dict[str, object]]:
    data = tomllib.loads(path.read_text(encoding="utf-8"))
    mappings = data.get("mapping", [])
    if not isinstance(mappings, list):
        raise ValueError("taxonomy 'mapping' must be a list")

    normalized: list[dict[str, object]] = []
    for entry in mappings:
        if not isinstance(entry, dict):
            raise ValueError("each mapping entry must be a table")
        category = str(entry.get("category", "")).strip()
        subcategory = str(entry.get("subcategory", "")).strip()
        tools = entry.get("tools", [])
        if not category or not subcategory:
            raise ValueError("mapping entry requires non-empty category and subcategory")
        if not isinstance(tools, list) or not tools:
            raise ValueError(f"mapping {category}.{subcategory} must contain non-empty tools list")
        normalized.append(
            {
                "category": category,
                "subcategory": subcategory,
                "tools": [str(t).strip() for t in tools if str(t).strip()],
            }
        )
    return normalized


def validate_unique_tool_assignments(mappings: list[dict[str, object]]) -> list[str]:
    assignments: dict[str, tuple[str, str]] = {}
    collisions: list[str] = []
    for entry in mappings:
        category = str(entry["category"])
        subcategory = str(entry["subcategory"])
        for tool in entry["tools"]:  # type: ignore[index]
            prev = assignments.get(tool)
            here = (category, subcategory)
            if prev is None:
                assignments[tool] = here
                continue
            if prev != here:
                collisions.append(
                    f"{tool}: assigned to {prev[0]}.{prev[1]} and {here[0]}.{here[1]}"
                )
    return sorted(set(collisions))


def render_rust_taxonomy_block(mappings: list[dict[str, object]]) -> str:
    tuples: list[tuple[str, str, str]] = []
    for entry in mappings:
        category = entry["category"]
        subcategory = entry["subcategory"]
        for tool in entry["tools"]:  # type: ignore[index]
            tuples.append((tool, category, subcategory))
    tuples.sort(key=lambda x: x[0])

    lines = [
        RS_BEGIN,
        "const EXPLICIT_TOOL_CATEGORY_SUBCATEGORY: &[(&str, &str, &str)] = &[",
    ]
    for tool, category, subcategory in tuples:
        lines.append(f'    ("{tool}", "{category}", "{subcategory}"),')
    lines.append("];")
    lines.append(RS_END)
    return "\n".join(lines)


def render_rust_subcategories_block(mappings: list[dict[str, object]]) -> str:
    lines = [RS_SUBCATS_BEGIN]
    for category, subcategories in taxonomy_subcategories_by_category(mappings):
        lines.append(f'        "{category}" => &[')
        for subcategory in subcategories:
            lines.append(f'            "{subcategory}",')
        lines.append("        ],")
    lines.append(RS_SUBCATS_END)
    return "\n".join(lines)


def replace_between_markers(content: str, begin: str, end: str, replacement: str) -> str:
    pattern = re.compile(rf"{re.escape(begin)}.*?{re.escape(end)}", re.DOTALL)
    if not pattern.search(content):
        raise ValueError(f"markers not found: {begin} ... {end}")
    return pattern.sub(replacement, content)


def extract_between_markers(content: str, begin: str, end: str) -> str:
    pattern = re.compile(rf"{re.escape(begin)}.*?{re.escape(end)}", re.DOTALL)
    match = pattern.search(content)
    if not match:
        raise ValueError(f"markers not found: {begin} ... {end}")
    return match.group(0)


def extract_class_method_names(stub_text: str, class_name: str) -> set[str]:
    class_pattern = re.compile(rf"^class\s+{re.escape(class_name)}\b.*?:\n", re.MULTILINE)
    matches = list(class_pattern.finditer(stub_text))
    if not matches:
        raise ValueError(f"class not found in stubs: {class_name}")

    methods: set[str] = set()
    for match in matches:
        start = match.end()
        next_class = re.search(r"^class\s+\w+\b.*?:\n", stub_text[start:], re.MULTILINE)
        end = start + next_class.start() if next_class else len(stub_text)
        class_body = stub_text[start:end]
        methods.update(re.findall(r"^\s+def\s+([a-zA-Z0-9_]+)\s*\(", class_body, re.MULTILINE))
    return methods


def render_stub_subcategory_block(mappings: list[dict[str, object]], stub_text: str) -> str:
    without_existing_auto = replace_between_markers(
        stub_text,
        STUB_AUTO_BEGIN,
        STUB_AUTO_END,
        f"{STUB_AUTO_BEGIN}\n{STUB_AUTO_END}",
    )
    existing_classes = set(
        re.findall(r"^class\s+(Wb[A-Za-z0-9_]+Subcategory)\b", without_existing_auto, re.MULTILINE)
    )

    lines = [STUB_AUTO_BEGIN]
    emitted_any = False
    for entry in mappings:
        category = str(entry["category"])
        subcategory = str(entry["subcategory"])
        class_name = stub_class_name(category, subcategory)
        if class_name in existing_classes:
            continue
        emitted_any = True
        lines.append(f"class {class_name}(WbToolSubcategory):")
        for tool in entry["tools"]:  # type: ignore[index]
            lines.append(f"    def {tool}(self, *args: Any, **kwargs: Any) -> Any: ...")
        lines.append("")

    if not emitted_any:
        lines.append("# No additional auto-generated subcategory classes required.")
    lines.append(STUB_AUTO_END)
    return "\n".join(lines)


def validate_stub_coverage(mappings: list[dict[str, object]], stub_path: Path) -> list[str]:
    stub_text = stub_path.read_text(encoding="utf-8")
    stub_text = replace_between_markers(
        stub_text,
        STUB_AUTO_BEGIN,
        STUB_AUTO_END,
        render_stub_subcategory_block(mappings, stub_text),
    )

    missing: list[str] = []
    for entry in mappings:
        category = str(entry["category"])
        subcategory = str(entry["subcategory"])
        class_name = stub_class_name(category, subcategory)
        methods = extract_class_method_names(stub_text, class_name)
        for tool in entry["tools"]:  # type: ignore[index]
            if tool not in methods:
                missing.append(f"{tool} (expected in {class_name})")

    return sorted(missing)


def _discover_source_crates(tool_ids: set[str]) -> dict[str, str]:
    source_hits: dict[str, set[str]] = {tool_id: set() for tool_id in tool_ids}
    crate_paths = {
        "wbtools_oss": ROOT / "crates" / "wbtools_oss",
        "wbtools_pro": ROOT.parent / "wbtools_pro",
    }
    id_pattern = re.compile(r'id:\s*"([a-zA-Z0-9_\-]+)"')

    for crate_name, crate_path in crate_paths.items():
        if not crate_path.exists():
            continue
        for rs_file in crate_path.rglob("*.rs"):
            try:
                content = rs_file.read_text(encoding="utf-8", errors="ignore")
            except OSError:
                continue
            for tool_id in id_pattern.findall(content):
                if tool_id in source_hits:
                    source_hits[tool_id].add(crate_name)

    out: dict[str, str] = {}
    for tool_id, hits in source_hits.items():
        if hits == {"wbtools_oss"}:
            out[tool_id] = "wbtools_oss"
        elif hits == {"wbtools_pro"}:
            out[tool_id] = "wbtools_pro"
        elif hits:
            out[tool_id] = "both"
        else:
            out[tool_id] = "unknown"
    return out


def _discover_license_tiers_from_source(tool_ids: set[str]) -> dict[str, str]:
    crate_paths = [
        ROOT / "crates" / "wbtools_oss",
        ROOT.parent / "wbtools_pro",
    ]
    # Capture metadata/manifest declarations where id and license tier appear nearby.
    decl_pattern = re.compile(
        r'id:\s*"([a-zA-Z0-9_\-]+)".{0,1400}?license_tier:\s*LicenseTier::([A-Za-z]+)',
        re.DOTALL,
    )

    tiers: dict[str, str] = {}
    for crate_path in crate_paths:
        if not crate_path.exists():
            continue
        for rs_file in crate_path.rglob("*.rs"):
            try:
                content = rs_file.read_text(encoding="utf-8", errors="ignore")
            except OSError:
                continue
            for tool_id, tier in decl_pattern.findall(content):
                if tool_id in tool_ids:
                    tiers[tool_id] = tier.lower()
    return tiers


def build_resolved_payload(mappings: list[dict[str, object]]) -> dict[str, Any]:
    tool_ids = {
        str(tool)
        for entry in mappings
        for tool in entry["tools"]  # type: ignore[index]
    }
    source_crate_by_tool = _discover_source_crates(tool_ids)
    source_tier_by_tool = _discover_license_tiers_from_source(tool_ids)

    license_tier_by_tool: dict[str, str] = {}
    try:
        from whitebox_workflows import WbEnvironment  # type: ignore

        wbe = WbEnvironment()
        detailed = wbe.list_tools_detailed(include_locked=True)
        for item in detailed:
            if not isinstance(item, dict):
                continue
            tool_id = item.get("id")
            if isinstance(tool_id, str) and tool_id in tool_ids:
                tier = item.get("license_tier")
                if isinstance(tier, str):
                    license_tier_by_tool[tool_id] = tier.lower()
    except Exception:
        pass

    tools_meta: dict[str, dict[str, str]] = {}
    for tool_id in sorted(tool_ids):
        source = source_crate_by_tool.get(tool_id, "unknown")
        tier = source_tier_by_tool.get(tool_id) or license_tier_by_tool.get(tool_id)
        if tier is None:
            if source == "wbtools_pro":
                tier = "pro"
            else:
                tier = "open"
        tools_meta[tool_id] = {
            "license_tier": tier,
            "source_crate": source,
        }

    return {
        "mapping": mappings,
        "tools": tools_meta,
    }


def main() -> int:
    parser = argparse.ArgumentParser(description="Sync tool taxonomy into API mappings")
    parser.add_argument("--apply", action="store_true", help="Write generated mapping into wb_environment.rs")
    parser.add_argument(
        "--export-json",
        type=str,
        default="",
        help="Optional path to export resolved taxonomy JSON for downstream consumers (e.g., plugin refresh tooling)",
    )
    parser.add_argument(
        "--export-qgis-json",
        type=str,
        default=str(QGIS_TAXONOMY_PATH),
        help="Path to export resolved taxonomy JSON into the QGIS plugin.",
    )
    parser.add_argument(
        "--export-r-json",
        type=str,
        default=str(R_TAXONOMY_PATH),
        help="Path to export resolved taxonomy JSON into the R package.",
    )
    parser.add_argument(
        "--check",
        action="store_true",
        help="Validate taxonomy, stub coverage, and generated mapping drift without modifying files",
    )
    args = parser.parse_args()

    mappings = load_taxonomy(TAXONOMY_PATH)

    rust_content = WB_ENV_PATH.read_text(encoding="utf-8")
    rust_expected = replace_between_markers(
        replace_between_markers(rust_content, RS_BEGIN, RS_END, render_rust_taxonomy_block(mappings)),
        RS_SUBCATS_BEGIN,
        RS_SUBCATS_END,
        render_rust_subcategories_block(mappings),
    )

    stub_content = STUB_PATH.read_text(encoding="utf-8")
    stub_expected = replace_between_markers(
        stub_content,
        STUB_AUTO_BEGIN,
        STUB_AUTO_END,
        render_stub_subcategory_block(mappings, stub_content),
    )

    collisions = validate_unique_tool_assignments(mappings)
    if collisions:
        print("ERROR: Duplicate/conflicting taxonomy assignments:")
        for msg in collisions:
            print(f"  - {msg}")
        return 1

    missing = validate_stub_coverage(mappings, STUB_PATH)
    if missing:
        print("ERROR: Missing methods in whitebox_workflows.pyi for mapped taxonomy tools:")
        for name in missing:
            print(f"  - {name}")
        print("Run: update stubs before syncing taxonomy.")
        return 1

    if args.apply:
        WB_ENV_PATH.write_text(rust_expected, encoding="utf-8")
        print(f"Updated {WB_ENV_PATH}")
        if stub_content != stub_expected:
            STUB_PATH.write_text(stub_expected, encoding="utf-8")
            print(f"Updated {STUB_PATH}")

    if args.check:
        current_rust = WB_ENV_PATH.read_text(encoding="utf-8")
        current_stub = STUB_PATH.read_text(encoding="utf-8")
        if current_rust != rust_expected:
            print("ERROR: taxonomy-generated Rust blocks drift detected in wb_environment.rs")
            print("Run: python scripts/sync_tool_taxonomy.py --apply")
            return 1
        if current_stub != stub_expected:
            print("ERROR: taxonomy-generated stub blocks drift detected in whitebox_workflows.pyi")
            print("Run: python scripts/sync_tool_taxonomy.py --apply")
            return 1
        print("Taxonomy mapping block is in sync.")

    if args.export_json:
        export_path = Path(args.export_json)
        if not export_path.is_absolute():
            export_path = ROOT / export_path
        export_path.parent.mkdir(parents=True, exist_ok=True)
        export_path.write_text(json.dumps(build_resolved_payload(mappings), indent=2), encoding="utf-8")
        print(f"Wrote taxonomy export: {export_path}")

    if args.export_qgis_json:
        qgis_path = Path(args.export_qgis_json)
        if not qgis_path.is_absolute():
            qgis_path = ROOT / qgis_path
        qgis_path.parent.mkdir(parents=True, exist_ok=True)
        qgis_path.write_text(json.dumps(build_resolved_payload(mappings), indent=2), encoding="utf-8")
        print(f"Wrote QGIS taxonomy export: {qgis_path}")

    if args.export_r_json:
        r_path = Path(args.export_r_json)
        if not r_path.is_absolute():
            r_path = ROOT / r_path
        r_path.parent.mkdir(parents=True, exist_ok=True)
        r_path.write_text(json.dumps(build_resolved_payload(mappings), indent=2), encoding="utf-8")
        print(f"Wrote R taxonomy export: {r_path}")

    if not args.apply and not args.export_json and not args.export_qgis_json and not args.export_r_json and not args.check:
        print("Taxonomy validation passed (no changes applied).")

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
