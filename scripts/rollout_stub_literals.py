#!/usr/bin/env python3
"""Roll out Literal[...] typing in whitebox_workflows.pyi.

This script looks for function parameters in the stub matching:
    param: str = "default"
and upgrades them to:
    param: Literal["a", "b", ...] = "default"
when `describe_tool(<function_name>)` reports enumerated `choices` for that
parameter and the default value is one of those choices.
"""

from __future__ import annotations

import argparse
import re
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
STUB_PATH = ROOT / "crates/wbw_python/whitebox_workflows/whitebox_workflows.pyi"

DEF_RE = re.compile(r"^(\s*def\s+(?P<name>[a-zA-Z0-9_]+)\((?P<params>.*)\)\s*->\s*.*)$")
STR_PARAM_RE = re.compile(r"\b(?P<param>[a-zA-Z_][a-zA-Z0-9_]*)\s*:\s*str\s*=\s*\"(?P<default>[^\"]*)\"")
LITERAL_PARAM_RE = re.compile(r"\b(?P<param>[a-zA-Z_][a-zA-Z0-9_]*)\s*:\s*Literal\[[^\]]+\]\s*=\s*\"(?P<default>[^\"]*)\"")


def ensure_literal_import(stub_text: str) -> str:
    if "from typing import" not in stub_text:
        return stub_text
    if "Literal" in stub_text:
        return stub_text
    return stub_text.replace("from typing import Any", "from typing import Any, Literal", 1)


def quote_literal_value(value: str) -> str:
    return '"' + value.replace('\\', '\\\\').replace('"', '\\"') + '"'


def normalize_choice_token(value: str) -> str:
    return "".join(c for c in value.lower() if c.isalnum())


def resolve_default_choice(default: str, choices: list[str]) -> str | None:
    if default in choices:
        return default
    norm_default = normalize_choice_token(default)
    if not norm_default:
        return None
    matches = [c for c in choices if normalize_choice_token(c) == norm_default]
    if len(matches) == 1:
        return matches[0]
    return None


def build_choices_lookup() -> dict[str, dict[str, list[str]]]:
    import whitebox_workflows as wb  # local dependency

    wbe = wb.WbEnvironment()
    out: dict[str, dict[str, list[str]]] = {}

    for tool in wbe.list_tools():
        try:
            desc = wbe.describe_tool(tool)
        except Exception:
            continue
        params = desc.get("params", []) if isinstance(desc, dict) else []
        pmap: dict[str, list[str]] = {}
        for p in params:
            if not isinstance(p, dict):
                continue
            name = p.get("name")
            choices = p.get("choices")
            if isinstance(name, str) and isinstance(choices, list) and choices:
                vals = [str(v) for v in choices]
                if all(v and all(c.isalnum() or c == "_" for c in v) for v in vals):
                    pmap[name] = vals
        if pmap:
            out[tool] = pmap
    return out


def transform_stub(
    stub_text: str,
    choices_lookup: dict[str, dict[str, list[str]]],
    *,
    fix_default_mismatch: bool = False,
) -> tuple[str, int, int]:
    updated_lines: list[str] = []
    replacements = 0
    mismatch_fixes = 0

    for line in stub_text.splitlines():
        m = DEF_RE.match(line)
        if not m:
            updated_lines.append(line)
            continue

        fn_name = m.group("name")
        params = m.group("params")
        param_choices = choices_lookup.get(fn_name)
        if not param_choices or "Literal[" in params:
            updated_lines.append(line)
            continue

        def repl(pm: re.Match[str]) -> str:
            nonlocal replacements, mismatch_fixes
            param = pm.group("param")
            default = pm.group("default")
            choices = param_choices.get(param)
            if not choices:
                return pm.group(0)
            resolved_default = resolve_default_choice(default, choices)
            if resolved_default is None:
                return pm.group(0)
            if default != resolved_default and not fix_default_mismatch:
                return pm.group(0)
            literal = "Literal[" + ", ".join(quote_literal_value(v) for v in choices) + "]"
            replacements += 1
            if default != resolved_default:
                mismatch_fixes += 1
            return f"{param}: {literal} = {quote_literal_value(resolved_default)}"

        new_line = STR_PARAM_RE.sub(repl, line)
        updated_lines.append(new_line)

    return "\n".join(updated_lines) + "\n", replacements, mismatch_fixes


def build_coverage_report(stub_text: str, choices_lookup: dict[str, dict[str, list[str]]]) -> dict[str, object]:
    total_def_lines = 0
    tool_lines_with_choices = 0
    literal_params_total = 0
    str_params_total = 0
    str_params_with_choices = 0
    str_params_convertible = 0
    str_params_default_not_in_choices = 0
    str_params_normalizable_mismatch = 0

    per_tool: dict[str, dict[str, int]] = {}

    for line in stub_text.splitlines():
        m = DEF_RE.match(line)
        if not m:
            continue
        total_def_lines += 1
        fn_name = m.group("name")
        params = m.group("params")
        param_choices = choices_lookup.get(fn_name, {})

        tool_stats = per_tool.setdefault(
            fn_name,
            {
                "literal": 0,
                "str": 0,
                "str_with_choices": 0,
                "str_convertible": 0,
                "str_default_not_in_choices": 0,
            },
        )

        if param_choices:
            tool_lines_with_choices += 1

        for lm in LITERAL_PARAM_RE.finditer(params):
            literal_params_total += 1
            tool_stats["literal"] += 1

        for sm in STR_PARAM_RE.finditer(params):
            str_params_total += 1
            tool_stats["str"] += 1
            pname = sm.group("param")
            default = sm.group("default")
            choices = param_choices.get(pname)
            if choices:
                str_params_with_choices += 1
                tool_stats["str_with_choices"] += 1
                if default in choices:
                    str_params_convertible += 1
                    tool_stats["str_convertible"] += 1
                else:
                    str_params_default_not_in_choices += 1
                    tool_stats["str_default_not_in_choices"] += 1
                    if resolve_default_choice(default, choices) is not None:
                        str_params_normalizable_mismatch += 1

    tools_with_literal = sum(1 for s in per_tool.values() if s["literal"] > 0)
    tools_with_convertible_remaining = sum(1 for s in per_tool.values() if s["str_convertible"] > 0)

    top_remaining = sorted(
        (
            (name, stats["str_convertible"])
            for name, stats in per_tool.items()
            if stats["str_convertible"] > 0
        ),
        key=lambda t: (-t[1], t[0]),
    )[:20]

    return {
        "total_def_lines": total_def_lines,
        "tool_lines_with_choices": tool_lines_with_choices,
        "literal_params_total": literal_params_total,
        "str_params_total": str_params_total,
        "str_params_with_choices": str_params_with_choices,
        "str_params_convertible": str_params_convertible,
        "str_params_default_not_in_choices": str_params_default_not_in_choices,
        "str_params_normalizable_mismatch": str_params_normalizable_mismatch,
        "tools_with_literal": tools_with_literal,
        "tools_with_convertible_remaining": tools_with_convertible_remaining,
        "top_remaining": top_remaining,
    }


def main() -> int:
    parser = argparse.ArgumentParser(description="Roll out Literal typing in whitebox_workflows.pyi")
    parser.add_argument("--check", action="store_true", help="Only check for pending updates")
    parser.add_argument("--report", action="store_true", help="Print rollout coverage report")
    parser.add_argument(
        "--fix-default-mismatch",
        action="store_true",
        help="Allow conversion when default normalizes to exactly one enum choice",
    )
    args = parser.parse_args()

    text = STUB_PATH.read_text(encoding="utf-8")
    text = ensure_literal_import(text)
    lookup = build_choices_lookup()
    new_text, replacement_count, mismatch_fix_count = transform_stub(
        text,
        lookup,
        fix_default_mismatch=args.fix_default_mismatch,
    )

    if args.report:
        report = build_coverage_report(new_text, lookup)
        print("Literal rollout coverage")
        print(f"  def lines: {report['total_def_lines']}")
        print(f"  tool lines with discoverable choices: {report['tool_lines_with_choices']}")
        print(f"  literal params currently typed: {report['literal_params_total']}")
        print(f"  remaining str params: {report['str_params_total']}")
        print(f"  remaining str params with choices: {report['str_params_with_choices']}")
        print(f"  remaining directly convertible params: {report['str_params_convertible']}")
        print(f"  remaining blocked by default mismatch: {report['str_params_default_not_in_choices']}")
        print(f"  of blocked mismatches, auto-fixable by normalization: {report['str_params_normalizable_mismatch']}")
        print(f"  tools with at least one Literal param: {report['tools_with_literal']}")
        print(f"  tools with convertible params remaining: {report['tools_with_convertible_remaining']}")
        remaining = report["top_remaining"]
        if remaining:
            print("  top tools with convertible params remaining:")
            for tool_name, count in remaining:
                print(f"    - {tool_name}: {count}")
        else:
            print("  no directly convertible params remain")

    if args.check:
        if new_text != STUB_PATH.read_text(encoding="utf-8"):
            print("stub literals out of date")
            return 1
        print("stub literals in sync")
        return 0

    if new_text != STUB_PATH.read_text(encoding="utf-8"):
        STUB_PATH.write_text(new_text, encoding="utf-8")
        msg = f"updated {STUB_PATH} ({replacement_count} replacement(s))"
        if mismatch_fix_count:
            msg += f", including {mismatch_fix_count} default mismatch fix(es)"
        print(msg)
    else:
        print("no stub updates needed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
