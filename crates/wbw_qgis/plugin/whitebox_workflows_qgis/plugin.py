from __future__ import annotations

import json

from .diagnostics import diagnostics_text, gather_runtime_diagnostics
from .host_api import (
    open_processing_algorithm_dialog,
    qgis_major_version,
    qgis_version_string,
    register_dock_widget,
    register_plugin_action,
    register_provider,
    unregister_dock_widget,
    unregister_plugin_action,
    unregister_provider,
)
from .panel import WhiteboxDockPanel, summarize_catalog
from .provider import WhiteboxProcessingProvider

try:
    from qgis.PyQt.QtGui import QAction
    from qgis.PyQt.QtCore import QSettings
    from qgis.PyQt.QtWidgets import QMessageBox
except Exception:  # pragma: no cover
    class QAction:  # type: ignore[override]
        def __init__(self, *_args, **_kwargs):
            self.triggered = _Signal()

    class QMessageBox:  # type: ignore[override]
        @staticmethod
        def information(*_args, **_kwargs):
            return None

    class QSettings:  # type: ignore[override]
        def value(self, *_args, **_kwargs):
            return ""

        def setValue(self, *_args, **_kwargs):
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
        self._panel_action = None
        self._dock_panel = None
        self._recent_tool_ids: list[str] = []
        self._favorite_tool_ids: list[str] = []
        self._max_recent_tools = 8
        self._settings_key_recent = "whitebox_workflows/recent_tools"
        self._settings_key_favorites = "whitebox_workflows/favorite_tools"

    def initGui(self):
        # QGIS 4 is the primary target; avoid hard-fail in unknown hosts.
        major = qgis_major_version()
        if major not in (0, 4):
            return

        if not register_provider(self.iface, self.provider):
            return
        self._provider_registered = True

        self._load_recent_tools()
        self._load_favorite_tools()

        self._install_panel()
        self._install_actions()
        self._refresh_catalog(silent=True)

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

        panel_action = QAction("Show Whitebox Panel", self.iface.mainWindow())
        panel_action.triggered.connect(self._toggle_panel)
        if register_plugin_action(self.iface, panel_action, self._menu_label):
            self._panel_action = panel_action

    def _install_panel(self):
        panel = WhiteboxDockPanel(self.iface.mainWindow())
        panel.on_refresh(self._refresh_catalog)
        panel.on_diagnostics(self._show_diagnostics)
        panel.on_open_tool(self._open_tool_from_panel)
        panel.on_open_recent_tool(self._open_tool_from_recent)
        panel.on_open_favorite_tool(self._open_tool_from_favorite)
        panel.on_add_favorite(self._add_selected_favorite)
        panel.on_remove_favorite(self._remove_selected_favorite)
        if register_dock_widget(self.iface, panel):
            self._dock_panel = panel

    def _open_tool_from_panel(self, tool_id: str):
        provider_id = self.provider.id()
        opened = open_processing_algorithm_dialog(self.iface, provider_id, tool_id)
        if opened:
            self._record_recent_tool(tool_id)
            self._notify_info(f"Opening tool: {tool_id}")
        else:
            self._notify_warning(
                f"Unable to open dialog for {tool_id}; host processing API not available."
            )

    def _open_tool_from_recent(self, tool_id: str):
        self._open_tool_from_panel(tool_id)

    def _open_tool_from_favorite(self, tool_id: str):
        self._open_tool_from_panel(tool_id)

    def _record_recent_tool(self, tool_id: str):
        if tool_id in self._recent_tool_ids:
            self._recent_tool_ids.remove(tool_id)
        self._recent_tool_ids.insert(0, tool_id)
        if len(self._recent_tool_ids) > self._max_recent_tools:
            self._recent_tool_ids = self._recent_tool_ids[: self._max_recent_tools]
        if self._dock_panel is not None:
            self._dock_panel.set_recent_tools(self._recent_tool_ids)
        self._save_recent_tools()

    def _add_selected_favorite(self, *_args):
        if self._dock_panel is None:
            return
        tool_id = self._dock_panel.selected_result_tool_id()
        if not tool_id:
            self._notify_warning("Select a tool in the results list to add a favorite.")
            return
        if tool_id in self._favorite_tool_ids:
            self._notify_info(f"Already a favorite: {tool_id}")
            return
        self._favorite_tool_ids.append(tool_id)
        self._save_favorite_tools()
        self._dock_panel.set_favorites(self._favorite_tool_ids)
        self._notify_info(f"Added favorite: {tool_id}")

    def _remove_selected_favorite(self, *_args):
        if self._dock_panel is None:
            return
        tool_id = self._dock_panel.selected_favorite_tool_id()
        if not tool_id:
            self._notify_warning("Select a favorite entry to remove.")
            return
        self._favorite_tool_ids = [t for t in self._favorite_tool_ids if t != tool_id]
        self._save_favorite_tools()
        self._dock_panel.set_favorites(self._favorite_tool_ids)
        self._notify_info(f"Removed favorite: {tool_id}")

    def _load_recent_tools(self):
        try:
            settings = QSettings()
            raw = settings.value(self._settings_key_recent, "")
            if not raw:
                return
            parsed = json.loads(str(raw))
            if not isinstance(parsed, list):
                return
            cleaned = [str(x) for x in parsed if str(x).strip()]
            self._recent_tool_ids = cleaned[: self._max_recent_tools]
        except Exception:
            self._recent_tool_ids = []

    def _load_favorite_tools(self):
        try:
            settings = QSettings()
            raw = settings.value(self._settings_key_favorites, "")
            if not raw:
                return
            parsed = json.loads(str(raw))
            if not isinstance(parsed, list):
                return
            cleaned = [str(x) for x in parsed if str(x).strip()]
            self._favorite_tool_ids = cleaned
        except Exception:
            self._favorite_tool_ids = []

    def _save_recent_tools(self):
        try:
            settings = QSettings()
            settings.setValue(self._settings_key_recent, json.dumps(self._recent_tool_ids))
        except Exception:
            pass

    def _save_favorite_tools(self):
        try:
            settings = QSettings()
            settings.setValue(self._settings_key_favorites, json.dumps(self._favorite_tool_ids))
        except Exception:
            pass

    def _toggle_panel(self, *_args):
        panel = self._dock_panel
        if panel is None:
            return
        is_visible = getattr(panel, "isVisible", None)
        set_visible = getattr(panel, "setVisible", None)
        if callable(is_visible) and callable(set_visible):
            set_visible(not bool(is_visible()))

    def _show_diagnostics(self, *_args):
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

    def _refresh_catalog(self, *_args, silent: bool = False):
        try:
            catalog = self.provider.refresh_catalog(regenerate_help=True)
            refresh_algorithms = getattr(self.provider, "refreshAlgorithms", None)
            if callable(refresh_algorithms):
                refresh_algorithms()
        except Exception as exc:
            self._notify_warning(f"Catalog refresh failed: {exc}")
            return

        available, locked = summarize_catalog(catalog)

        payload = gather_runtime_diagnostics(
            include_pro=self.provider.include_pro,
            tier=self.provider.tier,
        )
        caps = payload.get("capabilities") if isinstance(payload, dict) else None
        effective_tier = "unknown"
        if isinstance(caps, dict):
            effective_tier = str(caps.get("effective_tier", "unknown"))

        if self._dock_panel is not None:
            self._dock_panel.set_catalog(catalog)
            self._dock_panel.set_favorites(self._favorite_tool_ids)
            self._dock_panel.set_recent_tools(self._recent_tool_ids)
            self._dock_panel.update_state(
                status=str(payload.get("status", "unknown")),
                requested_tier=self.provider.tier,
                effective_tier=effective_tier,
                available_count=available,
                locked_count=locked,
                qgis_version=qgis_version_string(),
            )

        if not silent:
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
        self._save_recent_tools()
        self._save_favorite_tools()
        if self._dock_panel is not None:
            unregister_dock_widget(self.iface, self._dock_panel)
            self._dock_panel = None
        if self._panel_action is not None:
            unregister_plugin_action(self.iface, self._panel_action, self._menu_label)
            self._panel_action = None
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