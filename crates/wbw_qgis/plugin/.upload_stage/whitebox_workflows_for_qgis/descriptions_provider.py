"""
Curated parameter descriptions for Whitebox Workflows QGIS plugin.

Provides enriched parameter labels and tooltips for tools where legacy help
is unavailable. Descriptions are stored in JSON format and loaded on-demand.

Description data is sourced in priority order:
  1. Installed whitebox_workflows package (whitebox_workflows/descriptions/)
     via wbw.get_all_descriptions_json() — stays in sync with the installed
     backend version without requiring a plugin update.
  2. Plugin-bundled descriptions/ directory — backward-compatible fallback for
     older backend versions that do not yet ship their own descriptions.

See docs/internal/parameter_descriptions_curation_guide.md for guidelines
on creating and maintaining parameter descriptions.
"""

import json
from pathlib import Path
from typing import Optional


def _load_from_backend() -> dict:
    """Load curated descriptions from the installed whitebox_workflows package.

    Returns an empty dict if the backend is not available or does not yet
    ship descriptions.
    """
    try:
        import whitebox_workflows as _wbw
        get_fn = getattr(_wbw, "get_all_descriptions_json", None)
        if callable(get_fn):
            raw = get_fn()
            if raw and raw.strip() not in ("{}", ""):
                data = json.loads(raw)
                if isinstance(data, dict) and data:
                    return data
    except Exception:
        pass
    # Direct filesystem fallback (works without the compiled extension being
    # up-to-date, e.g. in editable dev installs)
    try:
        import whitebox_workflows as _wbw
        init_file = getattr(_wbw, "__file__", None)
        if init_file:
            desc_dir = Path(str(init_file)).parent / "descriptions"
            if desc_dir.is_dir():
                files = sorted(
                    desc_dir.glob("*.json"),
                    key=lambda p: (0 if p.name.startswith("auto_generated") else 1, p.name),
                )
                merged: dict = {}
                for jf in files:
                    try:
                        with open(jf, encoding="utf-8") as f:
                            merged.update(json.load(f))
                    except Exception:
                        pass
                if merged:
                    return merged
    except Exception:
        pass
    return {}


class DescriptionsProvider:
    """Provides curated parameter descriptions for tools."""

    def __init__(self):
        """Initialize descriptions provider."""
        self._descriptions: dict = {}
        self._load_descriptions()

    def _load_descriptions(self):
        """Load all JSON description files.

        Tries the installed whitebox_workflows package first (so descriptions
        stay in sync with the backend version), then falls back to the plugin's
        own bundled descriptions/ directory.
        """
        # 1. Backend package (preferred — version-synced)
        backend_data = _load_from_backend()
        if backend_data:
            self._descriptions = backend_data
            return

        # 2. Plugin-bundled fallback
        descriptions_dir = Path(__file__).parent / "descriptions"
        if not descriptions_dir.exists():
            descriptions_dir.mkdir(parents=True, exist_ok=True)
            return

        files = sorted(descriptions_dir.glob("*.json"), key=lambda p: p.name)
        ordered_files = sorted(
            files,
            key=lambda p: (0 if p.name == "auto_generated_tier1.json" else 1, p.name),
        )
        for json_file in ordered_files:
            try:
                with open(json_file, "r", encoding="utf-8") as f:
                    self._descriptions.update(json.load(f))
            except Exception as e:
                print(f"Error loading descriptions from {json_file}: {e}")

    def get_parameter_label(self, tool_id: str, param_name: str) -> Optional[str]:
        """Get enriched label for a parameter.

        Args:
            tool_id: The tool ID (e.g., 'spatial_join')
            param_name: The parameter name (e.g., 'mode')

        Returns:
            Rich label text suitable for QGIS parameter display, or None
            if no curated description exists.
        """
        tool_desc = self._descriptions.get(tool_id)
        if not isinstance(tool_desc, dict):
            return None
        params = tool_desc.get("parameters")
        if not isinstance(params, dict):
            return None
        return params.get(param_name, {}).get("label")

    def get_parameter_tooltip(self, tool_id: str, param_name: str) -> Optional[str]:
        """Get additional tooltip guidance for a parameter.

        Args:
            tool_id: The tool ID
            param_name: The parameter name

        Returns:
            Tooltip text for complex/non-obvious parameters, or None.
        """
        tool_desc = self._descriptions.get(tool_id)
        if not isinstance(tool_desc, dict):
            return None
        params = tool_desc.get("parameters")
        if not isinstance(params, dict):
            return None
        return params.get(param_name, {}).get("tooltip")

    def get_tool_description(self, tool_id: str) -> Optional[str]:
        """Get tool-level description.

        Args:
            tool_id: The tool ID

        Returns:
            Brief tool summary, or None if not defined.
        """
        tool_desc = self._descriptions.get(tool_id)
        if not isinstance(tool_desc, dict):
            return None
        return tool_desc.get("description")

    def has_curated_descriptions(self, tool_id: str) -> bool:
        """Check if a tool has any curated descriptions."""
        tool_desc = self._descriptions.get(tool_id)
        return isinstance(tool_desc, dict) and isinstance(tool_desc.get("parameters"), dict)


# Global instance - lazy-loaded on first access
_global_descriptions_provider: Optional[DescriptionsProvider] = None


def get_descriptions_provider() -> DescriptionsProvider:
    """Get global descriptions provider instance."""
    global _global_descriptions_provider
    if _global_descriptions_provider is None:
        _global_descriptions_provider = DescriptionsProvider()
    return _global_descriptions_provider
