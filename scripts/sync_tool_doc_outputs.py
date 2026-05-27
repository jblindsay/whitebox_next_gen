#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
import re
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
STUB_PATH = ROOT / "crates/wbw_python/whitebox_workflows/whitebox_workflows.pyi"
TAXONOMY_PATH = ROOT / "crates/wbw_python/tool_taxonomy.resolved.json"
WB_ENV_PATH = ROOT / "crates/wbw_python/src/wb_environment.rs"
DOCS_GLOB = "crates/wbw_python/docs/tools_*.md"

SECTION_RE = re.compile(r"(^### [^\n]+\n)(.*?)(?=^### |\Z)", re.MULTILINE | re.DOTALL)
DEF_RE = re.compile(r"^\s*def\s+([a-zA-Z0-9_]+)\((.*?)\)\s*->\s*([^:]+):", re.MULTILINE)


@dataclass(frozen=True)
class Param:
    name: str
    annotation: str
    has_default: bool = False


@dataclass(frozen=True)
class Signature:
    name: str
    params: tuple[Param, ...]
    return_type: str
    output_labels: tuple[str, ...] = ()


def split_top_level(value: str, delimiter: str = ",") -> list[str]:
    parts: list[str] = []
    start = 0
    square = 0
    paren = 0
    brace = 0
    for index, char in enumerate(value):
        if char == "[":
            square += 1
        elif char == "]":
            square = max(square - 1, 0)
        elif char == "(":
            paren += 1
        elif char == ")":
            paren = max(paren - 1, 0)
        elif char == "{":
            brace += 1
        elif char == "}":
            brace = max(brace - 1, 0)
        elif char == delimiter and square == 0 and paren == 0 and brace == 0:
            parts.append(value[start:index].strip())
            start = index + 1
    tail = value[start:].strip()
    if tail:
        parts.append(tail)
    return parts


def load_tool_ids() -> set[str]:
    data = json.loads(TAXONOMY_PATH.read_text(encoding="utf-8"))
    mappings = data.get("mapping", [])
    tool_ids: set[str] = set()
    for entry in mappings:
        if not isinstance(entry, dict):
            continue
        for tool in entry.get("tools", []):
            tool_name = str(tool).strip()
            if tool_name:
                tool_ids.add(tool_name)
    return tool_ids


def load_tool_paths() -> dict[str, tuple[str, str]]:
    data = json.loads(TAXONOMY_PATH.read_text(encoding="utf-8"))
    mappings = data.get("mapping", [])
    tool_paths: dict[str, tuple[str, str]] = {}
    for entry in mappings:
        if not isinstance(entry, dict):
            continue
        category = str(entry.get("category", "")).strip()
        subcategory = str(entry.get("subcategory", "")).strip()
        for tool in entry.get("tools", []):
            tool_name = str(tool).strip()
            if tool_name:
                tool_paths[tool_name] = (category, subcategory)
    return tool_paths


def parse_params(param_text: str) -> tuple[Param, ...]:
    params: list[Param] = []
    for raw_part in split_top_level(param_text):
        part = raw_part.strip()
        if not part or part == "self" or part.startswith("*"):
            continue
        before_default, _, _default = part.partition("=")
        name_text, sep, annotation = before_default.partition(":")
        name = name_text.strip()
        if not name:
            continue
        params.append(Param(name=name, annotation=annotation.strip() if sep else "Any", has_default=("=" in part)))
    return tuple(params)


def choose_best_signatures(tool_ids: set[str]) -> dict[str, Signature]:
    stub_text = STUB_PATH.read_text(encoding="utf-8")
    rust_signatures = parse_rust_signatures(tool_ids)
    best: dict[str, tuple[int, Signature]] = {}
    for match in DEF_RE.finditer(stub_text):
        name, param_text, return_type = match.groups()
        if name not in tool_ids:
            continue
        signature = Signature(name=name, params=parse_params(param_text), return_type=return_type.strip())
        score = score_signature(signature)
        current = best.get(name)
        if current is None or score > current[0]:
            best[name] = (score, signature)

    resolved = {name: sig for name, (_score, sig) in best.items()}
    for name, rust_signature in rust_signatures.items():
        current = resolved.get(name)
        if current is None or current.return_type == "Any":
            resolved[name] = rust_signature
    return resolved


def parse_rust_signatures(tool_ids: set[str]) -> dict[str, Signature]:
    rust_text = WB_ENV_PATH.read_text(encoding="utf-8")
    signatures: dict[str, Signature] = {}
    for name in tool_ids:
        pattern = re.compile(
            rf"fn\s+{re.escape(name)}\s*\((.*?)\)\s*->\s*PyResult<([^>]+)>\s*\{{(.*?)(?=^\s*///|\Z)",
            re.MULTILINE | re.DOTALL,
        )
        match = pattern.search(rust_text)
        if not match:
            continue
        param_block, return_type, body = match.groups()
        output_labels = tuple(
            normalize_output_label(label)
            for label in re.findall(r"let\s+([a-zA-Z0-9_]+)\s*=\s*extract_output_path_by_key\(", body)
        )
        signatures[name] = Signature(
            name=name,
            params=parse_rust_params(param_block),
            return_type=normalize_rust_return_type(return_type.strip()),
            output_labels=output_labels,
        )
    return signatures


def parse_rust_params(param_text: str) -> tuple[Param, ...]:
    params: list[Param] = []
    for raw_part in split_top_level(param_text):
        part = raw_part.strip()
        if not part or part == "&self":
            continue
        name_text, sep, annotation = part.partition(":")
        name = name_text.strip().lstrip("&")
        if not name:
            continue
        params.append(Param(name=name, annotation=normalize_rust_return_type(annotation.strip()) if sep else "Any"))
    return tuple(params)


def normalize_rust_return_type(type_text: str) -> str:
    value = type_text.strip()
    if value.startswith("(") and value.endswith(")"):
        inner = value[1:-1]
        parts = [normalize_rust_return_type(part) for part in split_top_level(inner)]
        return f"tuple[{', '.join(parts)}]"
    if value.startswith("Option<") and value.endswith(">"):
        inner = normalize_rust_return_type(value[7:-1])
        return f"{inner} | None"
    mapping = {
        "String": "str",
        "f64": "float",
        "f32": "float",
        "i64": "int",
        "i32": "int",
        "u64": "int",
        "u32": "int",
        "usize": "int",
        "bool": "bool",
    }
    return mapping.get(value, value)


def score_signature(signature: Signature) -> int:
    score = 0
    if signature.return_type != "Any":
        score += 1000
    score -= signature.return_type.count("Any") * 20
    score += sum(1 for param in signature.params if param.annotation != "Any")
    score -= sum(1 for param in signature.params if param.annotation == "Any")
    score += sum(2 for param in signature.params if is_output_param(param))
    return score


def split_return_types(return_type: str) -> list[str]:
    normalized = return_type.strip()
    lower = normalized.lower()
    if lower.startswith("tuple[") and normalized.endswith("]"):
        inner = normalized[6:-1]
        return split_top_level(inner)
    if lower.startswith("tuple["):
        inner = normalized[6: normalized.rfind("]")]
        return split_top_level(inner)
    return [normalized]


def is_output_param(param: Param) -> bool:
    name = param.name
    annotation = param.annotation.lower()
    if "bool" in annotation:
        return False
    if name in {"out_type", "output_as_polygons"}:
        return False
    if name.startswith("output_") and name not in {"output_path", "output_prefix", "output1", "output2"}:
        return False
    return (
        name == "output"
        or name == "output_path"
        or name == "output_prefix"
        or name == "out_html"
        or name == "report_path"
        or name.endswith("_output")
        or name.endswith("_output_path")
        or name.endswith("_report")
        or name.endswith("_report_path")
        or name.endswith("_summary")
        or name.endswith("_summary_path")
        or name in {"output1", "output2"}
    )


def normalize_output_label(name: str) -> str:
    label = name
    for suffix in ("_output_path", "_report_path", "_summary_path", "_output", "_path"):
        if label.endswith(suffix):
            label = label[: -len(suffix)]
            break
    if label == "output":
        return "result"
    if label == "out_html":
        return "html_report"
    if label == "output_prefix":
        return "output_prefix"
    return label


def fallback_output_label(type_name: str, index: int) -> str:
    cleaned = type_name.replace("| None", "").strip()
    base = cleaned
    if cleaned.startswith("tuple["):
        base = "tuple"
    elif cleaned.startswith("list["):
        base = "list"
    elif cleaned.startswith("dict["):
        base = "dict"
    elif cleaned.lower() == "str":
        base = "string"
    elif cleaned.lower() in {"int", "float"}:
        base = cleaned.lower()
    elif cleaned == "Any":
        base = "value"
    else:
        base = re.sub(r"[^a-zA-Z0-9]+", "_", cleaned).strip("_").lower() or "value"
    return f"{base}_{index + 1}"


def render_outputs_block(signature: Signature) -> str:
    return_types = split_return_types(signature.return_type)
    output_params = list(signature.output_labels) or [
        normalize_output_label(param.name) for param in signature.params if is_output_param(param)
    ]

    lines = ["**Outputs**", ""]
    if len(return_types) == 1:
        lines.append(f"- `return`: `{return_types[0]}`")
        return "\n".join(lines)

    lines.append(f"Returned as `{signature.return_type}` in this order:")
    lines.append("")
    for index, type_name in enumerate(return_types):
        label = output_params[index] if index < len(output_params) else fallback_output_label(type_name, index)
        lines.append(f"- `{label}`: `{type_name}`")
    return "\n".join(lines)


def normalize_sample_block(block: str, tool_name: str, tool_paths: dict[str, tuple[str, str]]) -> str:
    normalized = block.replace("env.", "wbe.")
    preferred = preferred_call_path(tool_name, tool_paths)
    normalized = re.sub(rf"\bwbe\.{re.escape(tool_name)}\(", f"{preferred}(", normalized)
    return normalized.rstrip()


def extract_sample_block(section_body: str) -> str | None:
    anchors = [
        section_body.find("**WbEnvironment usage**"),
        section_body.find("Example:"),
        section_body.find("**Example**"),
    ]
    anchors = [idx for idx in anchors if idx >= 0]
    if not anchors:
        return None
    block = section_body[min(anchors):].rstrip()
    block = re.sub(r"\n---\s*$", "", block).rstrip()
    return block or None


def has_sample_block(section_body: str) -> bool:
    return extract_sample_block(section_body) is not None


def normalize_existing_sample_block(section_body: str, tool_name: str, tool_paths: dict[str, tuple[str, str]]) -> str:
    sample_block = extract_sample_block(section_body)
    if not sample_block:
        return section_body
    normalized = normalize_sample_block(sample_block, tool_name, tool_paths)
    if normalized == sample_block.rstrip():
        return section_body
    start = section_body.find(sample_block)
    if start < 0:
        return section_body
    end = start + len(sample_block)
    return f"{section_body[:start]}{normalized}{section_body[end:]}"


def extract_sections(doc_text: str) -> dict[str, str]:
    sections: dict[str, str] = {}
    for match in SECTION_RE.finditer(doc_text):
        sections[extract_tool_name(match.group(1))] = match.group(2)
    return sections


def load_head_doc(path: Path) -> str | None:
    relative = path.relative_to(ROOT).as_posix()
    result = subprocess.run(
        ["git", "show", f"HEAD:{relative}"],
        cwd=ROOT,
        capture_output=True,
        text=True,
        check=False,
    )
    if result.returncode != 0:
        return None
    return result.stdout


def parse_section_signature_defaults(section_body: str) -> set[str]:
    match = re.search(r"```\n([a-zA-Z0-9_]+)\((.*?)\)\n```", section_body, re.DOTALL)
    if not match:
        return set()
    defaults: set[str] = set()
    for part in split_top_level(match.group(2)):
        before_default, sep, _default = part.partition("=")
        if not sep:
            continue
        name = before_default.strip().lstrip("*")
        if name:
            defaults.add(name)
    return defaults


def format_param_type(annotation: str, name: str) -> str:
    cleaned = annotation.strip().replace("&", "")
    lower_name = name.lower()

    if lower_name == "callback" or "Py<PyAny>" in cleaned:
        return "function"
    if lower_name in {"output", "output_path", "output_prefix", "report_path", "out_html"}:
        return "string"
    if lower_name.endswith("_path") or lower_name.endswith("_output_path"):
        return "string"
    if cleaned == "Any":
        return "string" if "path" in lower_name or lower_name.startswith("output") else "Any"
    if "Raster" in cleaned:
        return "Raster"
    if "Vector" in cleaned:
        return "Vector"
    if "Lidar" in cleaned:
        return "Lidar"
    if cleaned.startswith("list["):
        inner = cleaned[5:-1] if cleaned.endswith("]") else cleaned
        if "Raster" in inner:
            return "list[Raster]"
        if "Vector" in inner:
            return "list[Vector]"
        return cleaned.replace("str", "string").replace("|", "\\|")

    no_none = cleaned.replace("| None", "").replace("None |", "").strip()
    normalized = re.sub(r"\bstr\b", "string", no_none)
    return normalized.replace("|", "\\|")


def describe_param(name: str, annotation: str, required: str) -> str:
    clean_type = annotation.replace(" ", "")
    lower_name = name.lower()
    if lower_name in {"callback"}:
        return "Optional progress callback receiving JSON events."
    if lower_name in {"output", "output_path"}:
        return "Optional output path. If omitted, the result is returned in memory when supported."
    if lower_name.endswith("_output_path"):
        return f"Optional output path for `{name.removesuffix('_output_path')}`."
    if lower_name == "output_prefix":
        return "Optional output prefix for multi-product outputs."
    if lower_name.endswith("_path"):
        return f"Path value for `{name.removesuffix('_path')}`."
    if "Raster" in annotation:
        if lower_name == "dem":
            return "Input DEM raster."
        return f"Input raster for `{name}`."
    if "Vector" in annotation:
        return f"Input vector layer for `{name}`."
    if "Lidar" in annotation:
        return f"Input LiDAR dataset for `{name}`."
    if clean_type.startswith("list["):
        return f"List input for `{name}`."
    if "bool" in clean_type.lower():
        return f"Boolean option for `{name}`."
    if "int" in clean_type.lower() or "float" in clean_type.lower():
        return f"Numeric parameter for `{name}`."
    if "str" in clean_type.lower() or "string" in clean_type.lower():
        return f"String parameter for `{name}`."
    if required == "no":
        return f"Optional parameter `{name}`."
    return f"Parameter `{name}`."


def render_parameters_table(section_body: str, signature: Signature) -> str:
    pattern = re.compile(r"\nParameters:\n((?:- .*\n)+)", re.MULTILINE)
    match = pattern.search(section_body)
    if not match:
        return section_body

    defaults = parse_section_signature_defaults(section_body)
    signature_params = {param.name: param for param in signature.params}
    rows: list[str] = ["**Parameters**", "", "| Name | Type | Required | Description |", "|---|---|---|---|"]
    for raw_line in match.group(1).strip().splitlines():
        line = raw_line.strip()
        item_match = re.match(r"-\s+`?([^`]+?)`?\s*:\s*(.*)", line)
        if not item_match:
            continue
        name, description = item_match.groups()
        param = signature_params.get(name)
        annotation = format_param_type(param.annotation if param else "Any", name)
        required = "no"
        desc_lower = description.lower()
        if param is not None:
            has_default = param.has_default or name in defaults or "optional" in desc_lower or "if omitted" in desc_lower or "default" in desc_lower or "none" in param.annotation.lower()
            required = "no" if has_default else "yes"
        elif "optional" not in desc_lower and "if omitted" not in desc_lower and "default" not in desc_lower:
            required = "yes"
        rows.append(f"| `{name}` | {annotation} | {required} | {description} |")

    replacement = "\n" + "\n".join(rows) + "\n"
    return pattern.sub(replacement, section_body, count=1)


def normalize_existing_parameters_table(section_body: str) -> str:
    pattern = re.compile(
        r"\n\*\*Parameters\*\*\n\n\| Name \| Type \| Required \| Description \|\n\|---\|---\|---\|---\|\n((?:\|.*\n)+)",
        re.MULTILINE,
    )
    match = pattern.search(section_body)
    if not match:
        return section_body

    rows = ["**Parameters**", "", "| Name | Type | Required | Description |", "|---|---|---|---|"]
    changed = False
    for raw_line in match.group(1).splitlines():
        line = raw_line.strip()
        if not line.startswith("|"):
            continue
        cells = [cell.strip() for cell in line.strip("|").split("|")]
        if len(cells) < 4:
            rows.append(raw_line.rstrip())
            continue
        name = cells[0]
        description = cells[-1]
        required = cells[-2].lower()
        type_text = "|".join(cells[1:-2]).strip()
        normalized_type = format_param_type(type_text, name.strip("`"))
        normalized_required = required if required in {"yes", "no"} else "yes"
        rebuilt = f"| {name} | {normalized_type} | {normalized_required} | {description} |"
        rows.append(rebuilt)
        if rebuilt != raw_line:
            changed = True

    if not changed:
        return section_body
    replacement = "\n" + "\n".join(rows) + "\n"
    return pattern.sub(replacement, section_body, count=1)


def ensure_parameters_table(section_body: str, signature: Signature) -> str:
    updated = render_parameters_table(section_body, signature)
    if updated != section_body:
        return updated
    updated = normalize_existing_parameters_table(section_body)
    if updated != section_body:
        return updated
    if "**Parameters**" in section_body:
        return section_body

    defaults = parse_section_signature_defaults(section_body)
    rows: list[str] = ["**Parameters**", "", "| Name | Type | Required | Description |", "|---|---|---|---|"]
    for param in signature.params:
        if param.name == "self":
            continue
        annotation = format_param_type(param.annotation, param.name)
        has_default = param.has_default or param.name in defaults or "none" in param.annotation.lower()
        required = "no" if has_default else "yes"
        description = describe_param(param.name, param.annotation, required)
        rows.append(f"| `{param.name}` | {annotation} | {required} | {description} |")

    if len(rows) == 4:
        return section_body

    signature_block = re.search(r"\n```\n.*?\n```\n", section_body, re.DOTALL)
    insertion = "\n" + "\n".join(rows) + "\n"
    if signature_block:
        return f"{section_body[:signature_block.end()]}{insertion}{section_body[signature_block.end():]}"
    return f"{section_body.rstrip()}\n{insertion}"


def preferred_call_path(tool_name: str, tool_paths: dict[str, tuple[str, str]]) -> str:
    category, subcategory = tool_paths.get(tool_name, ("", ""))
    if not category:
        return f"wbe.{tool_name}"
    if subcategory == "general" or not subcategory:
        return f"wbe.{category}.{tool_name}"
    return f"wbe.{category}.{subcategory}.{tool_name}"


def sample_value_for_param(param: Param, index: int = 1) -> str | None:
    name = param.name
    annotation = param.annotation
    lower = name.lower()
    if lower == "callback":
        return None
    if lower in {"output", "output_path"}:
        return f'output_path="{name if lower != "output" else "result"}.tif"'
    if lower.endswith("_output_path"):
        stem = lower.removesuffix("_output_path")
        ext = "geojson" if "vector" in stem or "route" in stem or "sites" in stem or "zones" in stem or "polygons" in stem else "tif"
        return f'{name}="{stem}.{ext}"'
    if lower.endswith("_path"):
        return f'{name}="{lower.removesuffix("_path")}.dat"'
    if lower == "output_prefix":
        return 'output_prefix="output/result"'
    if "Raster" in annotation:
        if lower == "dem":
            return "dem"
        return name
    if "Vector" in annotation:
        return name
    if annotation.startswith("list["):
        inner = annotation[5:-1] if annotation.endswith("]") else annotation
        if "Raster" in inner:
            return f"[{name}_1, {name}_2]"
        if "Vector" in inner:
            return f"[{name}_1, {name}_2]"
        return f"[{name}_1, {name}_2]"
    if "bool" in annotation.lower():
        return None
    if "int" in annotation.lower():
        return f"{name}=1"
    if "float" in annotation.lower():
        return f"{name}=1.0"
    if "str" in annotation.lower():
        return f'{name}="value"'
    return name


def generated_assignment(signature: Signature) -> str:
    if len(split_return_types(signature.return_type)) == 1:
        return "result"
    labels = list(signature.output_labels)
    if labels:
        return ", ".join(labels)
    return ", ".join(fallback_output_label(type_name, index) for index, type_name in enumerate(split_return_types(signature.return_type)))


def render_generated_sample_block(tool_name: str, signature: Signature, tool_paths: dict[str, tuple[str, str]]) -> str:
    call = preferred_call_path(tool_name, tool_paths)
    args: list[str] = []
    for param in signature.params:
        value = sample_value_for_param(param)
        if value is None:
            continue
        args.append(value)

    assignment = generated_assignment(signature)
    if not args:
        body = f"{assignment} = {call}()"
        return f"**WbEnvironment usage**\n\n```python\n{body}\n```"

    multiline = len(args) > 2 or any("=" in arg for arg in args[1:])
    if multiline:
        lines = [f"{assignment} = {call}("]
        for arg in args:
            lines.append(f"    {arg},")
        lines.append(")")
        code = "\n".join(lines)
    else:
        code = f"{assignment} = {call}({', '.join(args)})"
    return f"**WbEnvironment usage**\n\n```python\n{code}\n```"


def extract_tool_name(heading_line: str) -> str:
    name = heading_line.removeprefix("###").strip()
    return name.strip("`").strip()


def replace_returns_block(section_body: str, outputs_block: str) -> str:
    returns_pattern = re.compile(r"\nReturns:\n(?:- .*\n)+(?:\n)?", re.MULTILINE)
    if returns_pattern.search(section_body):
        return returns_pattern.sub(f"\n{outputs_block}\n\n", section_body, count=1)
    return section_body


def inject_outputs_block(section_body: str, outputs_block: str) -> str:
    if "**Outputs**" in section_body:
        existing_pattern = re.compile(
            r"\n\*\*Outputs\*\*\n.*?(?=\n\*\*WbEnvironment usage\*\*|\nExample:|\n\*\*Example\*\*|\n```python|\n---\n|\Z)",
            re.DOTALL,
        )
        if existing_pattern.search(section_body):
            return existing_pattern.sub(f"\n{outputs_block}\n", section_body, count=1)
        return section_body

    updated = replace_returns_block(section_body, outputs_block)
    if updated != section_body:
        return updated

    anchor = re.search(r"\n(\*\*WbEnvironment usage\*\*|Example:|\*\*Example\*\*|```python)", section_body)
    if anchor:
        return f"{section_body[:anchor.start()]}\n\n{outputs_block}\n\n{section_body[anchor.start():].lstrip()}"

    stripped = section_body.rstrip()
    return f"{stripped}\n\n{outputs_block}\n"


def ensure_sample_block(section_body: str, sample_block: str | None, tool_name: str, signature: Signature, tool_paths: dict[str, tuple[str, str]]) -> str:
    if has_sample_block(section_body):
        return section_body

    block = normalize_sample_block(sample_block, tool_name, tool_paths) if sample_block else render_generated_sample_block(tool_name, signature, tool_paths)
    outputs_anchor = re.search(r"\n\*\*Outputs\*\*\n.*?(?=\n\*\*WbEnvironment usage\*\*|\nExample:|\n\*\*Example\*\*|\n```python|\n---\n|\Z)", section_body, re.DOTALL)
    if outputs_anchor:
        return f"{section_body[:outputs_anchor.end()].rstrip()}\n\n{block}\n"
    return f"{section_body.rstrip()}\n\n{block}\n"


def update_document(doc_path: Path, doc_text: str, signatures: dict[str, Signature]) -> tuple[str, int]:
    updates = 0
    head_doc = load_head_doc(doc_path)
    head_sections = extract_sections(head_doc) if head_doc else {}
    tool_paths = load_tool_paths()

    def replace_section(match: re.Match[str]) -> str:
        nonlocal updates
        heading = match.group(1)
        body = match.group(2)
        tool_name = extract_tool_name(heading)
        signature = signatures.get(tool_name)
        if signature is None:
            return match.group(0)
        new_body = ensure_parameters_table(body, signature)
        new_body = inject_outputs_block(new_body, render_outputs_block(signature))
        new_body = normalize_existing_sample_block(new_body, tool_name, tool_paths)
        sample_block = extract_sample_block(head_sections.get(tool_name, "")) if tool_name in head_sections else None
        new_body = ensure_sample_block(new_body, sample_block, tool_name, signature, tool_paths)
        if new_body != body:
            updates += 1
        new_body = new_body.rstrip("\n") + "\n\n"
        return f"{heading}{new_body}"

    return SECTION_RE.sub(replace_section, doc_text), updates


def main() -> int:
    parser = argparse.ArgumentParser(description="Sync explicit output types into shared tool docs.")
    parser.add_argument("--check", action="store_true", help="Exit non-zero if files would change.")
    args = parser.parse_args()

    tool_ids = load_tool_ids()
    signatures = choose_best_signatures(tool_ids)
    docs = sorted(ROOT.glob(DOCS_GLOB))

    changed_files: list[Path] = []
    updated_sections = 0
    for doc_path in docs:
        original = doc_path.read_text(encoding="utf-8")
        updated, section_updates = update_document(doc_path, original, signatures)
        if updated != original:
            changed_files.append(doc_path)
            updated_sections += section_updates
            if not args.check:
                doc_path.write_text(updated, encoding="utf-8")

    if args.check:
        if changed_files:
            print(f"Would update {len(changed_files)} files and {updated_sections} tool sections.")
            for path in changed_files:
                print(path.relative_to(ROOT))
            return 1
        print("Tool docs already include synced outputs.")
        return 0

    print(f"Updated {len(changed_files)} files and {updated_sections} tool sections.")
    for path in changed_files:
        print(path.relative_to(ROOT))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())