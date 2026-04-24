#!/usr/bin/env python3
from __future__ import annotations

import csv
import json
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
TRACKER = ROOT / "docs/performance/tool_parity_tracker.csv"
TAXONOMY = ROOT / "crates/wbw_python/tool_taxonomy.resolved.json"
WBTOOLS = ROOT / "crates/wbtools_oss/src/tools"


def collect_ng_from_taxonomy() -> set[str]:
    data = json.loads(TAXONOMY.read_text(encoding="utf-8"))
    ng: set[str] = set()
    for bucket in data.get("mapping", []):
        for tool in bucket.get("tools", []):
            if isinstance(tool, str) and tool:
                ng.add(tool.strip())
    return ng


def collect_ng_from_wbtools() -> set[str]:
    ng: set[str] = set()
    id_pattern = re.compile(r'id:\s*"([a-z0-9_\-{}]+)"')
    metadata_pattern = re.compile(r'\b[a-zA-Z0-9_]*tool_metadata\(\s*"([a-z0-9_\-{}]+)"')
    manifest_pattern = re.compile(r'\b[a-zA-Z0-9_]*tool_manifest\(\s*"([a-z0-9_\-{}]+)"')

    for path in WBTOOLS.rglob("*.rs"):
        text = path.read_text(encoding="utf-8", errors="ignore")
        for pat in (id_pattern, metadata_pattern, manifest_pattern):
            for m in pat.findall(text):
                if m:
                    ng.add(m)
    return ng


def main() -> int:
    ng_tools = collect_ng_from_taxonomy() | collect_ng_from_wbtools()

    rows: list[dict[str, str]] = []
    with TRACKER.open("r", newline="", encoding="utf-8") as f:
        reader = csv.DictReader(f)
        fieldnames = reader.fieldnames
        if not fieldnames:
            raise RuntimeError("Tracker CSV has no header")
        for row in reader:
            tool = (row.get("tool_name") or "").strip()
            row["exists_next_gen"] = "TRUE" if tool in ng_tools else "FALSE"
            rows.append(row)

    with TRACKER.open("w", newline="", encoding="utf-8") as f:
        writer = csv.DictWriter(f, fieldnames=fieldnames)
        writer.writeheader()
        writer.writerows(rows)

    print(f"Updated exists_next_gen flags for {len(rows)} rows")
    print(f"NG tools recognized: {len(ng_tools)}")

    # Spot-check a few known cases.
    lookup = {r.get('tool_name', ''): r for r in rows}
    for name in ("block_maximum", "block_minimum", "lidar_block_maximum", "lidar_idw_interpolation"):
        if name in lookup:
            print(f"{name}: exists_next_gen={lookup[name].get('exists_next_gen')}")

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
