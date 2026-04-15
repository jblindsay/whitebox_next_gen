from __future__ import annotations

from .bootstrap import get_runtime_capabilities, get_tool_catalog, load_whitebox_workflows


def discover_runtime(include_pro: bool = True, tier: str = "open") -> dict:
    return get_runtime_capabilities(include_pro=include_pro, tier=tier)


def discover_tool_catalog(include_pro: bool = True, tier: str = "open") -> list[dict]:
    catalog = get_tool_catalog(include_pro=include_pro, tier=tier)

    def _rank_value(item: dict) -> int:
        raw = item.get("display_default_rank")
        try:
            if raw is None:
                return 999_999
            return int(raw)
        except Exception:
            return 999_999

    return sorted(
        catalog,
        key=lambda item: (
            _rank_value(item),
            item.get("category", ""),
            item.get("display_name", ""),
            item.get("id", ""),
        ),
    )


def split_catalog_by_availability(catalog: list[dict]) -> tuple[list[dict], list[dict]]:
    available = [item for item in catalog if item.get("available")]
    locked = [item for item in catalog if not item.get("available")]
    return available, locked


def refresh_and_build_help(
    include_pro: bool = True,
    tier: str = "open",
    *,
    force: bool = False,
) -> tuple[list[dict], dict[str, str]]:
    """Discover the current tool catalog and generate help HTML files.

    Intended as a top-level convenience for the settings/refresh action.

    Returns:
        (catalog, help_index) where help_index maps tool_id → HTML file path.
    """
    from .help import generate_help_files

    catalog = discover_tool_catalog(include_pro=include_pro, tier=tier)
    wbw = load_whitebox_workflows()
    help_index = generate_help_files(wbw, catalog, force=force)
    return catalog, help_index