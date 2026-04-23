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


def ensure_literal_import(stub_text: str) -> str:
    if "from typing import" not in stub_text:
        return stub_text
    if "Literal" in stub_text:
        return stub_text
    return stub_text.replace("from typing import Any", "from typing import Any, Literal", 1)


def quote_literal_value(value: str) -> str:
    return '"' + value.replace('\\', '\\\\').replace('"', '\\"') + '"'


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


def transform_stub(stub_text: str, choices_lookup: dict[str, dict[str, list[str]]]) -> tuple[str, int]:
    updated_lines: list[str] = []
    replacements = 0

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
            nonlocal replacements
            param = pm.group("param")
            default = pm.group("default")
            choices = param_choices.get(param)
            if not choices:
                return pm.group(0)
            if default not in choices:
                return pm.group(0)
            literal = "Literal[" + ", ".join(quote_literal_value(v) for v in choices) + "]"
            replacements += 1
            return f"{param}: {literal} = {quote_literal_value(default)}"

        new_line = STR_PARAM_RE.sub(repl, line)
        updated_lines.append(new_line)

    return "\n".join(updated_lines) + "\n", replacements


def main() -> int:
    parser = argparse.ArgumentParser(description="Roll out Literal typing in whitebox_workflows.pyi")
    parser.add_argument("--check", action="store_true", help="Only check for pending updates")
    args = parser.parse_args()

    text = STUB_PATH.read_text(encoding="utf-8")
    text = ensure_literal_import(text)
    lookup = build_choices_lookup()
    new_text, replacement_count = transform_stub(text, lookup)

    if args.check:
        if new_text != STUB_PATH.read_text(encoding="utf-8"):
            print("stub literals out of date")
            return 1
        print("stub literals in sync")
        return 0

    if new_text != STUB_PATH.read_text(encoding="utf-8"):
        STUB_PATH.write_text(new_text, encoding="utf-8")
        print(f"updated {STUB_PATH} ({replacement_count} replacement(s))")
    else:
        print("no stub updates needed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
