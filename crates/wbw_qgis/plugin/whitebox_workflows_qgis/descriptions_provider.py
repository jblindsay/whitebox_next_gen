"""
Curated parameter descriptions for Whitebox Workflows QGIS plugin.

Provides enriched parameter labels and tooltips for tools where legacy help
is unavailable. Descriptions are stored in JSON format and loaded on-demand.

See docs/internal/parameter_descriptions_curation_guide.md for guidelines
on creating and maintaining parameter descriptions.
"""

import json
from pathlib import Path
from typing import Optional


class DescriptionsProvider:
    """Provides curated parameter descriptions for tools."""

    def __init__(self):
        """Initialize descriptions provider."""
        self._descriptions = {}
        self._load_descriptions()

    def _load_descriptions(self):
        """Load all JSON description files from descriptions directory."""
        descriptions_dir = Path(__file__).parent / "descriptions"
        if not descriptions_dir.exists():
            # Create empty descriptions directory
            descriptions_dir.mkdir(parents=True, exist_ok=True)
            return

        # Load all JSON files in descriptions directory
        for json_file in descriptions_dir.glob("*.json"):
            try:
                with open(json_file, "r", encoding="utf-8") as f:
                    data = json.load(f)
                    # Merge into main descriptions dict
                    self._descriptions.update(data)
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
        if tool_id not in self._descriptions:
            return None

        tool_desc = self._descriptions[tool_id]
        if "parameters" not in tool_desc:
            return None

        params = tool_desc["parameters"]
        if param_name not in params:
            return None

        return params[param_name].get("label")

    def get_parameter_tooltip(self, tool_id: str, param_name: str) -> Optional[str]:
        """Get additional tooltip guidance for a parameter.

        Args:
            tool_id: The tool ID
            param_name: The parameter name

        Returns:
            Tooltip text for complex/non-obvious parameters, or None.
        """
        if tool_id not in self._descriptions:
            return None

        tool_desc = self._descriptions[tool_id]
        if "parameters" not in tool_desc:
            return None

        params = tool_desc["parameters"]
        if param_name not in params:
            return None

        return params[param_name].get("tooltip")

    def get_tool_description(self, tool_id: str) -> Optional[str]:
        """Get tool-level description (for future use in help panels).

        Args:
            tool_id: The tool ID

        Returns:
            Brief tool summary, or None if not defined.
        """
        if tool_id not in self._descriptions:
            return None

        return self._descriptions[tool_id].get("description")

    def has_curated_descriptions(self, tool_id: str) -> bool:
        """Check if a tool has any curated descriptions."""
        return tool_id in self._descriptions and "parameters" in self._descriptions[tool_id]


# Global instance - lazy-loaded on first access
_global_descriptions_provider: Optional[DescriptionsProvider] = None


def get_descriptions_provider() -> DescriptionsProvider:
    """Get global descriptions provider instance."""
    global _global_descriptions_provider
    if _global_descriptions_provider is None:
        _global_descriptions_provider = DescriptionsProvider()
    return _global_descriptions_provider
