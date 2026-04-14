"""
Help documentation generation for the WbW QGIS plugin.

Help HTML is derived from WbEnvironment method docstrings (the same source used
by the previous Whitebox QGIS plugin) and from tool-manifest metadata obtained
via WbW-Py's catalog JSON APIs.  Files are generated on first plugin load and
regenerated whenever the user triggers a catalog refresh or a WbW-Py upgrade is
detected.

Generated files are stored in a user-writable cache directory and are never
bundled as static files.  This ensures help content always reflects the
installed version of WbW-Py.
"""
from __future__ import annotations

import inspect
import os
import re


# ---------------------------------------------------------------------------
# Cache directory
# ---------------------------------------------------------------------------

def get_help_cache_dir() -> str:
    """Return a writable directory for caching generated help HTML files.

    Tries QGIS's user settings directory first, then falls back to the
    user's home directory.  Creates the directory if it does not exist.
    """
    try:
        from qgis.core import QgsApplication  # type: ignore[import]
        base = os.path.join(
            QgsApplication.qgisSettingsDirPath(),
            "whitebox_workflows_qgis",
            "help",
        )
    except ImportError:
        base = os.path.join(
            os.path.expanduser("~"),
            ".whitebox_workflows_qgis",
            "help",
        )

    os.makedirs(base, exist_ok=True)
    return base


# ---------------------------------------------------------------------------
# Markdown → HTML conversion
# (Uses the optional `markdown` library if available, or a minimal fallback.)
# ---------------------------------------------------------------------------

def _convert_markdown(md_text: str) -> str:
    """Convert a markdown docstring to HTML.

    Tries the `markdown` library first, then falls back to very basic
    conversion.  The fallback handles paragraphs, code spans, #-headings,
    and bullet lists well enough for typical WbW docstrings.
    """
    try:
        import markdown as md_lib  # type: ignore[import]
        return md_lib.markdown(md_text)
    except ImportError:
        pass

    # -- minimal fallback --
    html_lines: list[str] = []
    in_list = False
    in_code_block = False

    for line in md_text.split("\n"):
        stripped = line.rstrip()

        if stripped.startswith("```"):
            if not in_code_block:
                in_code_block = True
                html_lines.append("<pre><code>")
            else:
                in_code_block = False
                html_lines.append("</code></pre>")
            continue

        if in_code_block:
            html_lines.append(stripped)
            continue

        # headings
        heading_m = re.match(r"^(#{1,4})\s+(.*)", stripped)
        if heading_m:
            level = min(len(heading_m.group(1)) + 1, 4)  # h2–h4 only
            html_lines.append(f"<h{level}>{heading_m.group(2)}</h{level}>")
            if in_list:
                html_lines.insert(-1, "</ul>")
                in_list = False
            continue

        # bullet lists
        if re.match(r"^[\*\-]\s+", stripped):
            if not in_list:
                html_lines.append("<ul>")
                in_list = True
            content = re.sub(r"^[\*\-]\s+", "", stripped)
            content = _inline_format(content)
            html_lines.append(f"<li>{content}</li>")
            continue

        if in_list and not stripped:
            html_lines.append("</ul>")
            in_list = False

        if stripped:
            html_lines.append(f"<p>{_inline_format(stripped)}</p>")

    if in_list:
        html_lines.append("</ul>")

    return "\n".join(html_lines)


def _inline_format(text: str) -> str:
    """Apply code span and bold/italic formatting within a text fragment."""
    # code span: `...`
    text = re.sub(r"`([^`]+)`", r"<code>\1</code>", text)
    # bold: **...**
    text = re.sub(r"\*\*(.+?)\*\*", r"<strong>\1</strong>", text)
    # italic: *...*
    text = re.sub(r"\*(.+?)\*", r"<em>\1</em>", text)
    return text


# ---------------------------------------------------------------------------
# Core help generation
# ---------------------------------------------------------------------------

BANNED_METHODS = frozenset(
    [
        "max_procs",
        "verbose",
        "working_directory",
        "version",
        "license_type",
        "check_in_license",
        "new_lidar",
        "read_lidar",
        "read_lidars",
        "write_lidar",
        "new_raster",
        "read_raster",
        "read_rasters",
        "write_raster",
        "new_vector",
        "read_vector",
        "read_vectors",
        "write_vector",
        "write_text",
        "categories",
        "domain_namespaces",
        "describe_tool",
        "list_tools",
        "run_tool",
        "run_tool_stream",
        "list_tool_catalog_json",
        "get_tool_metadata_json",
        "get_runtime_capabilities_json",
    ]
)


def _help_html_for_docstring(
    tool_id: str,
    display_name: str,
    docstring: str,
    is_pro: bool,
) -> str:
    """Render a single tool help HTML string from a docstring."""
    main_html = _convert_markdown(docstring.strip())

    # rewrite relative tool-help links to the online manual
    main_html = main_html.replace(
        "tool_help.md#",
        "https://www.whiteboxgeo.com/manual/wbw-user-manual/book/tool_help.html#",
    )

    pro_badge = (
        '<p><strong>PRO</strong> — This tool requires a Whitebox Workflows Pro license.</p>\n'
        if is_pro
        else ""
    )

    return f"""\
{pro_badge}\
{main_html}
<h2>Project Links</h2>
<div align="left">
    <a href="https://www.whiteboxgeo.com/whitebox-workflows-for-python/">WbW Homepage</a>
    <a href="https://www.whiteboxgeo.com/manual/wbw-user-manual/book/preface.html">User Manual</a>
    <a href="https://www.whiteboxgeo.com/whitebox-workflows/">Learn More</a>
</div>
"""


def _help_html_from_manifest(
    manifest: dict,
    is_pro: bool,
) -> str:
    """Build a minimal help page from manifest data (used when no docstring exists)."""
    tool_id: str = manifest.get("id", "")
    display_name: str = manifest.get("display_name", tool_id)
    summary: str = manifest.get("summary", "No description available.")
    params: list[dict] = manifest.get("params", [])

    pro_badge = (
        '<p><strong>PRO</strong> — This tool requires a Whitebox Workflows Pro license.</p>\n'
        if is_pro
        else ""
    )

    param_rows = ""
    for p in params:
        name = p.get("name", "")
        desc = p.get("description", "")
        req = "Required" if p.get("required", False) else "Optional"
        param_rows += f"<tr><td><code>{name}</code></td><td>{desc}</td><td>{req}</td></tr>\n"

    param_table = ""
    if param_rows:
        param_table = (
            "<h2>Parameters</h2>\n"
            "<table><thead><tr><th>Name</th><th>Description</th><th>Required</th></tr></thead>\n"
            f"<tbody>{param_rows}</tbody></table>\n"
        )

    return f"""\
{pro_badge}\
<p>{summary}</p>
{param_table}\
<h2>Project Links</h2>
<div align="left">
    <a href="https://www.whiteboxgeo.com/whitebox-workflows-for-python/">WbW Homepage</a>
    <a href="https://www.whiteboxgeo.com/manual/wbw-user-manual/book/preface.html">User Manual</a>
    <a href="https://www.whiteboxgeo.com/whitebox-workflows/">Learn More</a>
</div>
"""


# ---------------------------------------------------------------------------
# Public API — generate + cache
# ---------------------------------------------------------------------------

def generate_help_files(wbw_module, catalog: list[dict], *, force: bool = False) -> dict[str, str]:
    """Generate help HTML files for all tools in the catalog.

    Args:
        wbw_module: The imported `whitebox_workflows` module.
        catalog: Tool-catalog list from `list_tool_catalog_json()`.
        force: Re-generate even if a cached file already exists.

    Returns:
        Mapping of tool_id → absolute path of the generated HTML file.
    """
    cache_dir = get_help_cache_dir()
    result: dict[str, str] = {}

    # Build a lookup of docstrings from WbEnvironment methods
    docstrings: dict[str, str] = {}
    wbe_class = getattr(wbw_module, "WbEnvironment", None)
    if wbe_class is not None:
        for method_name, method_obj in inspect.getmembers(wbe_class):
            if method_name.startswith("__") or method_name in BANNED_METHODS:
                continue
            doc = inspect.getdoc(method_obj)
            if doc and len(doc.strip()) > 10:
                docstrings[method_name] = doc

    for item in catalog:
        tool_id: str = item.get("id", "")
        if not tool_id:
            continue

        display_name: str = item.get("display_name", tool_id)
        is_pro: bool = item.get("license_tier", "open") in ("pro", "enterprise")

        out_path = os.path.join(cache_dir, f"{tool_id}.html")
        result[tool_id] = out_path

        if not force and os.path.exists(out_path):
            continue

        if tool_id in docstrings:
            html = _help_html_for_docstring(
                tool_id,
                display_name,
                docstrings[tool_id],
                is_pro,
            )
        else:
            html = _help_html_from_manifest(item, is_pro)

        with open(out_path, "w", encoding="utf-8") as fh:
            fh.write(html)

    return result


def get_help_html(tool_id: str, catalog: list[dict] | None = None) -> str:
    """Return the help HTML string for a tool.

    Reads from the cache if available; otherwise generates and caches it first.
    Requires WbW-Py to be importable.
    """
    cache_dir = get_help_cache_dir()
    cached_path = os.path.join(cache_dir, f"{tool_id}.html")

    if os.path.exists(cached_path):
        with open(cached_path, encoding="utf-8") as fh:
            return fh.read()

    # Not cached — generate now
    from .bootstrap import load_whitebox_workflows, get_tool_catalog

    wbw = load_whitebox_workflows()
    if catalog is None:
        catalog = get_tool_catalog()

    manifest = next((item for item in catalog if item.get("id") == tool_id), None)
    if manifest is None:
        return f"<p>No help available for tool <code>{tool_id}</code>.</p>"

    is_pro = manifest.get("license_tier", "open") in ("pro", "enterprise")

    # Try docstring first
    wbe_class = getattr(wbw, "WbEnvironment", None)
    if wbe_class is not None:
        method = getattr(wbe_class, tool_id, None)
        if method is not None:
            doc = inspect.getdoc(method)
            if doc and len(doc.strip()) > 10:
                html = _help_html_for_docstring(tool_id, manifest.get("display_name", tool_id), doc, is_pro)
                with open(cached_path, "w", encoding="utf-8") as fh:
                    fh.write(html)
                return html

    html = _help_html_from_manifest(manifest, is_pro)
    with open(cached_path, "w", encoding="utf-8") as fh:
        fh.write(html)
    return html


def get_help_url(tool_id: str) -> str:
    """Return a file:// URL to the cached help HTML, or empty string."""
    cache_dir = get_help_cache_dir()
    cached_path = os.path.join(cache_dir, f"{tool_id}.html")
    if os.path.exists(cached_path):
        return cached_path.replace("\\", "/")
    return ""


def clear_help_cache() -> None:
    """Delete all cached help HTML files, forcing regeneration on next access."""
    cache_dir = get_help_cache_dir()
    for fname in os.listdir(cache_dir):
        if fname.endswith(".html"):
            try:
                os.remove(os.path.join(cache_dir, fname))
            except OSError:
                pass
