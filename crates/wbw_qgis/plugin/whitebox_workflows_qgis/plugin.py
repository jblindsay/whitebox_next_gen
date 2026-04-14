from __future__ import annotations

from .diagnostics import diagnostics_text, gather_runtime_diagnostics
from .host_api import (
    qgis_major_version,
    qgis_version_string,
    register_plugin_action,
    register_provider,
    unregister_plugin_action,
    unregister_provider,
)
from .provider import WhiteboxProcessingProvider

try:
    from qgis.PyQt.QtGui import QAction
    from qgis.PyQt.QtWidgets import QMessageBox
except Exception:  # pragma: no cover
    class QAction:  # type: ignore[override]
        def __init__(self, *_args, **_kwargs):
            self.triggered = _Signal()

    class QMessageBox:  # type: ignore[override]
        @staticmethod
        def information(*_args, **_kwargs):
            return None

    class _Signal:  # type: ignore[override]
        def connect(self, *_args, **_kwargs):
            return None


class WhiteboxWorkflowsPlugin:
    def __init__(self, iface):
        self.iface = iface
        self.provider = WhiteboxProcessingProvider()
        self._provider_registered = False
        self._menu_label = "&Whitebox Workflows"
        self._diagnostics_action = None
        self._refresh_action = None

    def initGui(self):
        # QGIS 4 is the primary target; avoid hard-fail in unknown hosts.
        major = qgis_major_version()
        if major not in (0, 4):
            return

        if not register_provider(self.iface, self.provider):
            return
        self._provider_registered = True

        self._install_actions()

        # Helpful startup message where the host exposes a message bar.
        msg = f"Whitebox Workflows provider loaded (QGIS {qgis_version_string() or 'unknown'})."
        try:
            bar = self.iface.messageBar()
            push = getattr(bar, "pushInfo", None)
            if push is not None:
                push("Whitebox Workflows", msg)
        except Exception:
            pass

    def _install_actions(self):
        diagnostics_action = QAction("Runtime Diagnostics", self.iface.mainWindow())
        diagnostics_action.triggered.connect(self._show_diagnostics)
        if register_plugin_action(self.iface, diagnostics_action, self._menu_label):
            self._diagnostics_action = diagnostics_action

        refresh_action = QAction("Refresh Catalog + Help", self.iface.mainWindow())
        refresh_action.triggered.connect(self._refresh_catalog)
        if register_plugin_action(self.iface, refresh_action, self._menu_label):
            self._refresh_action = refresh_action

    def _show_diagnostics(self):
        payload = gather_runtime_diagnostics(
            include_pro=self.provider.include_pro,
            tier=self.provider.tier,
        )
        text = diagnostics_text(payload)

        try:
            QMessageBox.information(
                self.iface.mainWindow(),
                "Whitebox Workflows Diagnostics",
                text,
            )
            return
        except Exception:
            pass

        try:
            bar = self.iface.messageBar()
            push = getattr(bar, "pushWarning", None)
            if push is not None:
                push("Whitebox Workflows", "Diagnostics unavailable as dialog; see logs.")
        except Exception:
            pass

    def _refresh_catalog(self):
        try:
            catalog = self.provider.refresh_catalog(regenerate_help=True)
            refresh_algorithms = getattr(self.provider, "refreshAlgorithms", None)
            if callable(refresh_algorithms):
                refresh_algorithms()
        except Exception as exc:
            self._notify_warning(f"Catalog refresh failed: {exc}")
            return

        available = 0
        locked = 0
        for item in catalog:
            if bool(item.get("locked", False)):
                locked += 1
            else:
                available += 1

        self._notify_info(
            f"Catalog refreshed: {available} available, {locked} locked tools."
        )

    def _notify_info(self, message: str):
        try:
            bar = self.iface.messageBar()
            push = getattr(bar, "pushInfo", None)
            if push is not None:
                push("Whitebox Workflows", message)
                return
        except Exception:
            pass

    def _notify_warning(self, message: str):
        try:
            bar = self.iface.messageBar()
            push = getattr(bar, "pushWarning", None)
            if push is not None:
                push("Whitebox Workflows", message)
                return
        except Exception:
            pass

    def unload(self):
        if self._refresh_action is not None:
            unregister_plugin_action(self.iface, self._refresh_action, self._menu_label)
            self._refresh_action = None
        if self._diagnostics_action is not None:
            unregister_plugin_action(self.iface, self._diagnostics_action, self._menu_label)
            self._diagnostics_action = None
        if not self._provider_registered:
            return
        if unregister_provider(self.iface, self.provider):
            self._provider_registered = False