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
import inspect
import re
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
STUB_PATH = ROOT / "crates/wbw_python/whitebox_workflows/whitebox_workflows.pyi"
WB_ENV_RS_PATH = ROOT / "crates/wbw_python/src/wb_environment.rs"

DEF_RE = re.compile(r"^(\s*def\s+(?P<name>[a-zA-Z0-9_]+)\((?P<params>.*)\)\s*->\s*.*)$")
STR_PARAM_RE = re.compile(r"\b(?P<param>[a-zA-Z_][a-zA-Z0-9_]*)\s*:\s*str\s*=\s*\"(?P<default>[^\"]*)\"")
LITERAL_PARAM_RE = re.compile(r"\b(?P<param>[a-zA-Z_][a-zA-Z0-9_]*)\s*:\s*Literal\[[^\]]+\]\s*=\s*\"(?P<default>[^\"]*)\"")
PLACEHOLDER_RE = re.compile(
    r"^(?P<indent>\s*)def\s+(?P<name>[a-zA-Z_][a-zA-Z0-9_]*)\(self,\s*\*args:\s*Any,\s*\*\*kwargs:\s*Any\)\s*->\s*Any:\s*\.\.\.\s*$"
)
FULL_DEF_RE = re.compile(
    r"^(?P<indent>\s*)def\s+(?P<name>[a-zA-Z_][a-zA-Z0-9_]*)\((?P<params>.*)\)\s*->\s*(?P<ret>[^:]+):\s*\.\.\.\s*$"
)
RUST_PYO3_SIGNATURE_RE = re.compile(r"#\[pyo3\(signature = \((?P<sig>.*)\)\)\]")
RUST_FN_RE = re.compile(r"^\s*pub\s+fn\s+(?P<name>[a-zA-Z_][a-zA-Z0-9_]*)\s*\(")


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


def fill_placeholders_from_existing_signatures(stub_text: str) -> tuple[str, int, int]:
    """Fill Any placeholders using unambiguous signatures already present in the stub.

    Returns: (updated_text, replacements, ambiguous_or_missing)
    """
    lines = stub_text.splitlines()

    signatures_by_name: dict[str, set[tuple[str, str]]] = {}
    for line in lines:
        m = FULL_DEF_RE.match(line)
        if not m:
            continue
        name = m.group("name")
        params = m.group("params")
        ret = m.group("ret").strip()
        # Exclude the placeholder form from sources.
        if params.strip() == "self, *args: Any, **kwargs: Any" and ret == "Any":
            continue
        signatures_by_name.setdefault(name, set()).add((params, ret))

    replacements = 0
    unresolved = 0
    out_lines: list[str] = []

    for line in lines:
        pm = PLACEHOLDER_RE.match(line)
        if not pm:
            out_lines.append(line)
            continue

        name = pm.group("name")
        indent = pm.group("indent")
        sigs = signatures_by_name.get(name, set())
        if len(sigs) != 1:
            unresolved += 1
            out_lines.append(line)
            continue

        params, ret = next(iter(sigs))
        out_lines.append(f"{indent}def {name}({params}) -> {ret}: ...")
        replacements += 1

    return "\n".join(out_lines) + "\n", replacements, unresolved


def split_top_level_csv(text: str) -> list[str]:
    parts: list[str] = []
    cur: list[str] = []
    depth_paren = 0
    depth_bracket = 0
    depth_brace = 0
    in_single = False
    in_double = False
    escape = False

    for ch in text:
        if escape:
            cur.append(ch)
            escape = False
            continue

        if ch == "\\" and (in_single or in_double):
            cur.append(ch)
            escape = True
            continue

        if in_single:
            cur.append(ch)
            if ch == "'":
                in_single = False
            continue
        if in_double:
            cur.append(ch)
            if ch == '"':
                in_double = False
            continue

        if ch == "'":
            cur.append(ch)
            in_single = True
            continue
        if ch == '"':
            cur.append(ch)
            in_double = True
            continue

        if ch == "(":
            depth_paren += 1
            cur.append(ch)
            continue
        if ch == ")":
            depth_paren -= 1
            cur.append(ch)
            continue
        if ch == "[":
            depth_bracket += 1
            cur.append(ch)
            continue
        if ch == "]":
            depth_bracket -= 1
            cur.append(ch)
            continue
        if ch == "{":
            depth_brace += 1
            cur.append(ch)
            continue
        if ch == "}":
            depth_brace -= 1
            cur.append(ch)
            continue

        if ch == "," and depth_paren == 0 and depth_bracket == 0 and depth_brace == 0:
            token = "".join(cur).strip()
            if token:
                parts.append(token)
            cur = []
            continue

        cur.append(ch)

    token = "".join(cur).strip()
    if token:
        parts.append(token)
    return parts


def normalize_rust_default(default_expr: str) -> str:
    expr = default_expr.strip()
    expr = expr.replace("f64::NEG_INFINITY", "-1.7976931348623157e308")
    expr = expr.replace("f64::INFINITY", "1.7976931348623157e308")
    expr = expr.replace("true", "True").replace("false", "False")
    expr = re.sub(r"\b(-?\d+)(?:u8|u16|u32|u64|usize|i8|i16|i32|i64|isize)\b", r"\1", expr)
    return expr


def build_rust_signature_lookup(rs_text: str) -> dict[str, set[str]]:
    pending_sig: str | None = None
    by_name: dict[str, set[str]] = {}

    for line in rs_text.splitlines():
        sm = RUST_PYO3_SIGNATURE_RE.search(line)
        if sm:
            pending_sig = sm.group("sig").strip()
            continue

        fm = RUST_FN_RE.match(line)
        if fm and pending_sig is not None:
            fn_name = fm.group("name")
            by_name.setdefault(fn_name, set()).add(pending_sig)
            pending_sig = None

    return by_name


def rust_sig_to_stub_params(sig: str) -> str:
    pieces = split_top_level_csv(sig)
    out = ["self"]
    for part in pieces:
        token = part.strip()
        if not token:
            continue
        if "=" in token:
            name, default = token.split("=", 1)
            out.append(f"{name.strip()}: Any = {normalize_rust_default(default)}")
        else:
            out.append(f"{token}: Any")
    return ", ".join(out)


def fill_placeholders_from_rust_signatures(stub_text: str, rs_text: str) -> tuple[str, int, int]:
    """Fill Any placeholders from unambiguous Rust pyo3 signatures.

    Returns: (updated_text, replacements, ambiguous_or_missing)
    """
    rust_lookup = build_rust_signature_lookup(rs_text)
    lines = stub_text.splitlines()
    out_lines: list[str] = []
    replacements = 0
    unresolved = 0

    for line in lines:
        pm = PLACEHOLDER_RE.match(line)
        if not pm:
            out_lines.append(line)
            continue

        name = pm.group("name")
        indent = pm.group("indent")
        sigs = rust_lookup.get(name, set())
        if len(sigs) != 1:
            unresolved += 1
            out_lines.append(line)
            continue

        sig = next(iter(sigs))
        params = rust_sig_to_stub_params(sig)
        out_lines.append(f"{indent}def {name}({params}) -> Any: ...")
        replacements += 1

    return "\n".join(out_lines) + "\n", replacements, unresolved


def render_python_default_literal(value: object) -> str:
    if value is None:
        return "None"
    if isinstance(value, bool):
        return "True" if value else "False"
    if isinstance(value, (int, float)):
        return repr(value)
    if isinstance(value, str):
        return quote_literal_value(value)
    if isinstance(value, (list, tuple, dict, set)):
        return repr(value)
    return repr(value)


def build_runtime_signature_lookup() -> dict[str, set[str]]:
    import whitebox_workflows as wb  # local dependency

    wbe = wb.WbEnvironment()
    by_name: dict[str, set[str]] = {}

    for tool in wbe.list_tools():
        fn = getattr(wbe, tool, None)
        if fn is None:
            continue
        try:
            sig = inspect.signature(fn)
        except (TypeError, ValueError):
            continue

        params_out = ["self"]
        skip = False
        for p in sig.parameters.values():
            if p.kind in (inspect.Parameter.VAR_POSITIONAL, inspect.Parameter.VAR_KEYWORD):
                skip = True
                break
            if p.kind in (inspect.Parameter.POSITIONAL_ONLY, inspect.Parameter.POSITIONAL_OR_KEYWORD, inspect.Parameter.KEYWORD_ONLY):
                if p.default is inspect._empty:
                    params_out.append(f"{p.name}: Any")
                else:
                    default_lit = render_python_default_literal(p.default)
                    params_out.append(f"{p.name}: Any = {default_lit}")

        if skip:
            continue

        by_name.setdefault(tool, set()).add(", ".join(params_out))

    return by_name


def fill_placeholders_from_runtime_signatures(stub_text: str) -> tuple[str, int, int]:
    """Fill Any placeholders from unambiguous runtime signatures.

    Returns: (updated_text, replacements, ambiguous_or_missing)
    """
    runtime_lookup = build_runtime_signature_lookup()
    lines = stub_text.splitlines()
    out_lines: list[str] = []
    replacements = 0
    unresolved = 0

    for line in lines:
        pm = PLACEHOLDER_RE.match(line)
        if not pm:
            out_lines.append(line)
            continue

        name = pm.group("name")
        indent = pm.group("indent")
        sigs = runtime_lookup.get(name, set())
        if len(sigs) != 1:
            unresolved += 1
            out_lines.append(line)
            continue

        params = next(iter(sigs))
        out_lines.append(f"{indent}def {name}({params}) -> Any: ...")
        replacements += 1

    return "\n".join(out_lines) + "\n", replacements, unresolved


def main() -> int:
    parser = argparse.ArgumentParser(description="Roll out Literal typing in whitebox_workflows.pyi")
    parser.add_argument("--check", action="store_true", help="Only check for pending updates")
    parser.add_argument("--report", action="store_true", help="Print rollout coverage report")
    parser.add_argument(
        "--fix-default-mismatch",
        action="store_true",
        help="Allow conversion when default normalizes to exactly one enum choice",
    )
    parser.add_argument(
        "--fill-any-from-existing",
        action="store_true",
        help="Replace '*args/**kwargs -> Any' placeholders using unambiguous existing signatures",
    )
    parser.add_argument(
        "--fill-any-from-rust-signatures",
        action="store_true",
        help="Replace '*args/**kwargs -> Any' placeholders using unambiguous wb_environment.rs pyo3 signatures",
    )
    parser.add_argument(
        "--fill-any-from-runtime-signatures",
        action="store_true",
        help="Replace '*args/**kwargs -> Any' placeholders using unambiguous WbEnvironment runtime signatures",
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

    placeholder_replacements = 0
    placeholder_unresolved = 0
    if args.fill_any_from_existing:
        new_text, placeholder_replacements, placeholder_unresolved = (
            fill_placeholders_from_existing_signatures(new_text)
        )

    rust_placeholder_replacements = 0
    rust_placeholder_unresolved = 0
    if args.fill_any_from_rust_signatures:
        rs_text = WB_ENV_RS_PATH.read_text(encoding="utf-8")
        new_text, rust_placeholder_replacements, rust_placeholder_unresolved = (
            fill_placeholders_from_rust_signatures(new_text, rs_text)
        )

    runtime_placeholder_replacements = 0
    runtime_placeholder_unresolved = 0
    if args.fill_any_from_runtime_signatures:
        new_text, runtime_placeholder_replacements, runtime_placeholder_unresolved = (
            fill_placeholders_from_runtime_signatures(new_text)
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
        if args.fill_any_from_existing:
            print(f"  placeholder Any signatures replaced this run: {placeholder_replacements}")
            print(f"  placeholder Any signatures unresolved (ambiguous/missing): {placeholder_unresolved}")
        if args.fill_any_from_rust_signatures:
            print(
                "  placeholder Any signatures replaced from Rust this run: "
                f"{rust_placeholder_replacements}"
            )
            print(
                "  placeholder Any signatures unresolved from Rust (ambiguous/missing): "
                f"{rust_placeholder_unresolved}"
            )
        if args.fill_any_from_runtime_signatures:
            print(
                "  placeholder Any signatures replaced from runtime this run: "
                f"{runtime_placeholder_replacements}"
            )
            print(
                "  placeholder Any signatures unresolved from runtime (ambiguous/missing): "
                f"{runtime_placeholder_unresolved}"
            )

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
        if args.fill_any_from_existing:
            msg += f", plus {placeholder_replacements} placeholder signature fill(s)"
        if args.fill_any_from_rust_signatures:
            msg += f", plus {rust_placeholder_replacements} Rust-derived placeholder fill(s)"
        if args.fill_any_from_runtime_signatures:
            msg += f", plus {runtime_placeholder_replacements} runtime-derived placeholder fill(s)"
        print(msg)
    else:
        print("no stub updates needed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
