from __future__ import annotations

import os

from .algorithm import build_algorithms
from .bootstrap import load_whitebox_workflows
from .discovery import discover_tool_catalog

try:
    from qgis.core import QgsProcessingProvider
    from qgis.PyQt.QtGui import QIcon
except ImportError:  # pragma: no cover
    class QgsProcessingProvider:  # type: ignore[override]
        pass

    class QIcon:  # type: ignore[override]
        def __init__(self, *_args, **_kwargs):
            pass


class WhiteboxProcessingProvider(QgsProcessingProvider):
    def __init__(self, include_pro: bool = True, tier: str = "open"):
        super().__init__()
        self._include_pro = include_pro
        self._tier = tier
        self._catalog: list[dict] = []
        self._help_index: dict[str, str] = {}  # tool_id -> cached html path

    def id(self):
        return "whitebox_workflows"

    def name(self):
        return "Whitebox Workflows"

    def longName(self):
        return "Whitebox Workflows"

    def icon(self):
        base_dir = os.path.dirname(__file__)
        candidates = (
            os.path.join(base_dir, "icons", "WbW.png"),
            os.path.join(base_dir, "icons", "WbW.svg"),
        )
        for path in candidates:
            if os.path.exists(path):
                return QIcon(path)
        return QIcon()

    def load(self):
        self.refresh_catalog()
        return True

    def unload(self):
        self._catalog = []
        self._help_index = {}

    def loadAlgorithms(self):
        self.refresh_catalog()
        for alg in build_algorithms(self, self._catalog):
            self.addAlgorithm(alg)
        return None

    def refresh_catalog(self, *, regenerate_help: bool = False) -> list[dict]:
        """Refresh the tool catalog and optionally regenerate help HTML files.

        Args:
            regenerate_help: Force-regenerate all help HTML files even if they
                already exist in the cache.  Use True after a WbW-Py upgrade.
        """
        self._catalog = discover_tool_catalog(
            include_pro=self._include_pro, tier=self._tier
        )
        self._generate_help(force=regenerate_help)
        return self._catalog

    def _generate_help(self, *, force: bool = False) -> None:
        """Generate help HTML files for the current catalog in the background."""
        if not self._catalog:
            return
        try:
            from .help import generate_help_files
            wbw = load_whitebox_workflows()
            self._help_index = generate_help_files(wbw, self._catalog, force=force)
        except Exception:  # never block the provider from loading
            pass

    def help_path_for_tool(self, tool_id: str) -> str:
        """Return the cached help HTML path for *tool_id*, or empty string."""
        return self._help_index.get(tool_id, "")

    @property
    def catalog(self) -> list[dict]:
        return list(self._catalog)

    @property
    def include_pro(self) -> bool:
        return bool(self._include_pro)

    @property
    def tier(self) -> str:
        return str(self._tier)