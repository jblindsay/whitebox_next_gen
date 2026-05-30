#!/usr/bin/env python3
"""Runtime metadata coverage report for Whitebox Next Gen.

Reports catalog-wide parameter metadata quality using the active
whitebox_workflows runtime in the current Python environment.
"""

from __future__ import annotations

import argparse
import json
from typing import Any

import whitebox_workflows as wbw


ALIAS_PAIRS = [
    ("d8_pntr", "d8_pointer"),
    ("output", "output_path"),
    ("pour_pts", "pour_points"),
]


def as_json(value: Any) -> Any:
    if isinstance(value, str):
        return json.loads(value)
    return value


def main() -> int:
    parser = argparse.ArgumentParser(description="Report runtime schema coverage metrics")
    parser.add_argument(
        "--sample-limit",
        type=int,
        default=25,
        help="Maximum number of sample tool IDs to print per category",
    )
    parser.add_argument(
        "--strict",
        action="store_true",
        help="Return non-zero if any coverage/contract issue is detected",
    )
    parser.add_argument(
        "--json",
        action="store_true",
        help="Emit report as JSON",
    )
    args = parser.parse_args()

    tool_ids = wbw.list_tools()

    missing_schema_tools: list[str] = []
    missing_desc_tools: list[str] = []
    callback_tools: list[str] = []
    path_param_tools: list[str] = []
    alias_duplicate_tools: list[tuple[str, str, str]] = []

    for tool_id in tool_ids:
        md = as_json(wbw.get_tool_metadata_json(tool_id))
        params = md.get("params", [])
        names = [p.get("name") for p in params if isinstance(p, dict)]

        if any(not isinstance((p.get("schema") if isinstance(p, dict) else None), dict) for p in params):
            missing_schema_tools.append(tool_id)

        if any(not (p.get("description") or "").strip() for p in params if isinstance(p, dict)):
            missing_desc_tools.append(tool_id)

        if "callback" in names:
            callback_tools.append(tool_id)

        if any(isinstance(n, str) and n.endswith("_path") for n in names):
            path_param_tools.append(tool_id)

        for left, right in ALIAS_PAIRS:
            if left in names and right in names:
                alias_duplicate_tools.append((tool_id, left, right))

    lim = max(args.sample_limit, 0)
    report = {
        "tool_count": len(tool_ids),
        "missing_schema_tool_count": len(missing_schema_tools),
        "missing_desc_tool_count": len(missing_desc_tools),
        "callback_tool_count": len(callback_tools),
        "path_param_tool_count": len(path_param_tools),
        "alias_duplicate_count": len(alias_duplicate_tools),
        "sample_missing_schema": missing_schema_tools[:lim],
        "sample_missing_desc": missing_desc_tools[:lim],
        "sample_callback": callback_tools[:lim],
        "sample_path_params": path_param_tools[:lim],
        "sample_alias_duplicates": alias_duplicate_tools[:lim],
    }

    if args.json:
        print(json.dumps(report, indent=2, sort_keys=True))
    else:
        for key in (
            "tool_count",
            "missing_schema_tool_count",
            "missing_desc_tool_count",
            "callback_tool_count",
            "path_param_tool_count",
            "alias_duplicate_count",
        ):
            print(key, report[key])
        for key in (
            "sample_missing_schema",
            "sample_missing_desc",
            "sample_callback",
            "sample_path_params",
            "sample_alias_duplicates",
        ):
            print(key, report[key])

    if args.strict:
        if (
            report["missing_schema_tool_count"]
            or report["missing_desc_tool_count"]
            or report["callback_tool_count"]
            or report["path_param_tool_count"]
            or report["alias_duplicate_count"]
        ):
            return 2

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
