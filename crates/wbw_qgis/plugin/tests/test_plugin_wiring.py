import os
import sys
import unittest
from unittest.mock import patch


_PLUGIN_ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), ".."))
if _PLUGIN_ROOT not in sys.path:
    sys.path.insert(0, _PLUGIN_ROOT)

from whitebox_workflows_qgis import plugin  # noqa: E402


class _FakeIface:
    def mainWindow(self):
        return None


class _FakePanel:
    def __init__(self, *_args, **_kwargs):
        self._on_refresh = None
        self._on_diagnostics = None
        self._on_session_banner_clicked = None

    def on_refresh(self, callback):
        self._on_refresh = callback

    def on_diagnostics(self, callback):
        self._on_diagnostics = callback

    def on_open_tool(self, _callback):
        return None

    def on_open_recent_tool(self, _callback):
        return None

    def on_open_favorite_tool(self, _callback):
        return None

    def on_add_favorite(self, _callback):
        return None

    def on_remove_favorite(self, _callback):
        return None

    def on_toggle_selected_favorite(self, _callback):
        return None

    def on_remove_selected_favorite_shortcut(self, _callback):
        return None

    def on_move_favorite_up(self, _callback):
        return None

    def on_move_favorite_down(self, _callback):
        return None

    def on_clear_favorites(self, _callback):
        return None

    def on_clear_recents(self, _callback):
        return None

    def on_quick_open_toggled(self, _callback):
        return None

    def on_filter_state_changed(self, _callback):
        return None

    def on_search_state_changed(self, _callback):
        return None

    def on_focus_area_changed(self, _callback):
        return None

    def on_session_banner_clicked(self, callback):
        self._on_session_banner_clicked = callback

    def on_tool_context_menu(self, _callback):
        return None

    def set_quick_open_enabled(self, _enabled):
        return None

    def set_show_available_enabled(self, _enabled):
        return None

    def set_show_locked_enabled(self, _enabled):
        return None

    def set_search_text(self, _text):
        return None

    def set_focus_area(self, _area):
        return None


class PluginPanelWiringTests(unittest.TestCase):
    def test_install_panel_wires_refresh_callback_to_plugin_handler(self):
        iface = _FakeIface()
        instance = plugin.WhiteboxWorkflowsPlugin(iface)

        calls = []
        instance._refresh_catalog = lambda *_a, **_k: calls.append("refresh")

        with patch.object(plugin, "WhiteboxDockPanel", _FakePanel), patch.object(
            plugin, "register_dock_widget", lambda _iface, _panel: True
        ):
            instance._install_panel()

        self.assertIsNotNone(instance._dock_panel)

        panel_obj = instance._dock_panel
        self.assertIsNotNone(panel_obj._on_refresh)

        panel_obj._on_refresh()

        self.assertEqual(calls, ["refresh"])

    def test_install_panel_wires_diagnostics_callbacks_to_plugin_handler(self):
        iface = _FakeIface()
        instance = plugin.WhiteboxWorkflowsPlugin(iface)

        calls = []
        instance._show_diagnostics = lambda *_a, **_k: calls.append("diagnostics")

        with patch.object(plugin, "WhiteboxDockPanel", _FakePanel), patch.object(
            plugin, "register_dock_widget", lambda _iface, _panel: True
        ):
            instance._install_panel()

        self.assertIsNotNone(instance._dock_panel)

        panel_obj = instance._dock_panel
        self.assertIsNotNone(panel_obj._on_diagnostics)
        self.assertIsNotNone(panel_obj._on_session_banner_clicked)

        panel_obj._on_diagnostics()
        panel_obj._on_session_banner_clicked()

        self.assertEqual(calls, ["diagnostics", "diagnostics"])


if __name__ == "__main__":
    unittest.main()
