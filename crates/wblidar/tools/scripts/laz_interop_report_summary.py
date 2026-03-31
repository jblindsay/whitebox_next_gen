#!/usr/bin/env python3
"""Render a markdown summary from a wblidar LAZ interoperability JSON report."""

from __future__ import annotations

import argparse
import json
from pathlib import Path


def _bool_to_str(value: object) -> str:
    if isinstance(value, bool):
        return "true" if value else "false"
    return "unknown"


def build_markdown(report: dict, source_name: str) -> str:
    tools = report.get("tools", {})
    summary = report.get("summary", {})
    profiles = report.get("profiles", [])
    policy_errors = report.get("policy_errors", [])

    lines: list[str] = []
    lines.append("## LAZ Interoperability Summary")
    lines.append("")
    lines.append(f"Source report: {source_name}")
    lines.append("")
    lines.append("### Run Summary")
    lines.append("")
    lines.append(f"- status: {summary.get('status', 'unknown')}")
    lines.append(f"- strict_mode: {report.get('strict_mode', 'unknown')}")
    lines.append(f"- executed_profiles: {summary.get('executed_profiles', 0)}")
    lines.append(f"- failed_profiles: {summary.get('failed_profiles', 0)}")
    lines.append(f"- generated_profiles: {summary.get('generated_profiles', 0)}")
    lines.append(f"- fixture_profiles: {summary.get('fixture_profiles', 0)}")
    lines.append(
        f"- required_min_fixture_profiles: {summary.get('required_min_fixture_profiles', 'none')}"
    )
    lines.append(
        f"- fixture_profile_requirement_met: {summary.get('fixture_profile_requirement_met', 'unknown')}"
    )
    lines.append(f"- policy_error_count: {summary.get('policy_error_count', 0)}")
    lines.append(f"- pdal_available: {_bool_to_str(tools.get('pdal_available'))}")
    lines.append(f"- lasinfo_available: {_bool_to_str(tools.get('lasinfo_available'))}")
    lines.append("")

    lines.append("### Profile Results")
    lines.append("")
    lines.append("| Source | PDRF | Extra Bytes | Internal Read | lasinfo | pdal info | Status |")
    lines.append("|---|---:|---:|---:|---|---|---|")

    for profile in profiles:
        source = profile.get("source", "unknown")
        pdrf = profile.get("pdrf", "?")
        extra = profile.get("extra_bytes_per_point", "?")
        internal = profile.get("internal_read_count", "?")
        lasinfo = profile.get("lasinfo", "unknown")
        pdal_info = profile.get("pdal_info", "unknown")
        status = profile.get("status", "unknown")
        lines.append(f"| {source} | {pdrf} | {extra} | {internal} | {lasinfo} | {pdal_info} | {status} |")

    failed = [p for p in profiles if p.get("status") == "failed"]
    if failed:
        lines.append("")
        lines.append("### Failures")
        lines.append("")
        for profile in failed:
            file_name = profile.get("file", "unknown")
            lines.append(f"- {file_name}")
            for err in profile.get("errors", []):
                lines.append(f"  - {err}")

    if policy_errors:
        lines.append("")
        lines.append("### Policy Errors")
        lines.append("")
        for err in policy_errors:
            lines.append(f"- {err}")

    lines.append("")
    return "\n".join(lines)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("report", type=Path, help="Path to interop JSON report")
    parser.add_argument(
        "--output",
        type=Path,
        required=True,
        help="Path to write markdown summary",
    )
    args = parser.parse_args()

    data = json.loads(args.report.read_text(encoding="utf-8"))
    markdown = build_markdown(data, args.report.name)

    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(markdown, encoding="utf-8")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
