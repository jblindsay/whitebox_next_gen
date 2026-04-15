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

    def isVisible(self):
        return True

    def width(self):
        return 340

    def show_available_enabled(self):
        return True

    def show_locked_enabled(self):
        return True

    def search_text(self):
        return ""

    def focus_area(self):
        return "search"


class PluginPanelWiringTests(unittest.TestCase):
    def test_load_panel_ui_state_coerces_string_values(self):
        iface = _FakeIface()
        instance = plugin.WhiteboxWorkflowsPlugin(iface)

        class _FakeSettings:
            def value(self, key, default=None):
                values = {
                    instance._settings_key_panel_visible: "false",
                    instance._settings_key_panel_width: "250",
                    instance._settings_key_show_available: "0",
                    instance._settings_key_show_locked: "yes",
                    instance._settings_key_search_text: "flow accumulation",
                    instance._settings_key_focus_area: "favorites",
                }
                return values.get(key, default)

        with patch.object(plugin, "QSettings", lambda: _FakeSettings()):
            instance._load_panel_ui_state()

        self.assertFalse(instance._panel_visible)
        self.assertEqual(instance._panel_width, 260)
        self.assertFalse(instance._panel_show_available)
        self.assertTrue(instance._panel_show_locked)
        self.assertEqual(instance._panel_search_text, "flow accumulation")
        self.assertEqual(instance._panel_focus_area, "favorites")

    def test_load_panel_ui_state_uses_defaults_on_settings_error(self):
        iface = _FakeIface()
        instance = plugin.WhiteboxWorkflowsPlugin(iface)

        class _BrokenSettings:
            def value(self, *_args, **_kwargs):
                raise RuntimeError("settings unavailable")

        with patch.object(plugin, "QSettings", lambda: _BrokenSettings()):
            instance._load_panel_ui_state()

        self.assertTrue(instance._panel_visible)
        self.assertEqual(instance._panel_width, 340)
        self.assertTrue(instance._panel_show_available)
        self.assertTrue(instance._panel_show_locked)
        self.assertEqual(instance._panel_search_text, "")
        self.assertEqual(instance._panel_focus_area, "search")

    def test_save_panel_ui_state_persists_panel_values_with_width_floor(self):
        iface = _FakeIface()
        instance = plugin.WhiteboxWorkflowsPlugin(iface)

        class _PanelState:
            def isVisible(self):
                return False

            def width(self):
                return 120

            def show_available_enabled(self):
                return False

            def show_locked_enabled(self):
                return True

            def search_text(self):
                return "channel heads"

            def focus_area(self):
                return "results"

        class _FakeSettings:
            def __init__(self):
                self.values = {}

            def setValue(self, key, value):
                self.values[key] = value

        settings = _FakeSettings()
        instance._dock_panel = _PanelState()

        with patch.object(plugin, "QSettings", lambda: settings):
            instance._save_panel_ui_state()

        self.assertEqual(settings.values[instance._settings_key_panel_visible], False)
        self.assertEqual(settings.values[instance._settings_key_panel_width], 260)
        self.assertEqual(settings.values[instance._settings_key_show_available], False)
        self.assertEqual(settings.values[instance._settings_key_show_locked], True)
        self.assertEqual(settings.values[instance._settings_key_search_text], "channel heads")
        self.assertEqual(settings.values[instance._settings_key_focus_area], "results")

    def test_toggle_panel_flips_visibility_and_saves_state(self):
        iface = _FakeIface()
        instance = plugin.WhiteboxWorkflowsPlugin(iface)

        class _TogglePanel:
            def __init__(self):
                self.visible = True
                self.calls = []

            def isVisible(self):
                return self.visible

            def setVisible(self, value):
                self.calls.append(bool(value))
                self.visible = bool(value)

        panel_obj = _TogglePanel()
        instance._dock_panel = panel_obj

        save_calls = []
        instance._save_panel_ui_state = lambda *_a, **_k: save_calls.append("saved")

        instance._toggle_panel()

        self.assertEqual(panel_obj.calls, [False])
        self.assertEqual(save_calls, ["saved"])

    def test_toggle_panel_noops_when_panel_missing(self):
        iface = _FakeIface()
        instance = plugin.WhiteboxWorkflowsPlugin(iface)

        save_calls = []
        instance._save_panel_ui_state = lambda *_a, **_k: save_calls.append("saved")

        instance._dock_panel = None
        instance._toggle_panel()

        self.assertEqual(save_calls, [])

    def test_toggle_panel_noops_when_visibility_methods_missing(self):
        iface = _FakeIface()
        instance = plugin.WhiteboxWorkflowsPlugin(iface)

        class _PanelWithoutVisibilityMethods:
            pass

        instance._dock_panel = _PanelWithoutVisibilityMethods()

        save_calls = []
        instance._save_panel_ui_state = lambda *_a, **_k: save_calls.append("saved")

        instance._toggle_panel()

        self.assertEqual(save_calls, [])

    def test_install_actions_registers_menu_actions_with_expected_handlers(self):
        iface = _FakeIface()
        instance = plugin.WhiteboxWorkflowsPlugin(iface)

        class _FakeSignal:
            def __init__(self):
                self.callback = None

            def connect(self, callback):
                self.callback = callback

        class _FakeAction:
            def __init__(self, text, parent):
                self.text = text
                self.parent = parent
                self.triggered = _FakeSignal()

        registered = []

        def _register_action(_iface, action, menu_label):
            registered.append((action, menu_label))
            return True

        with patch.object(plugin, "QAction", _FakeAction), patch.object(
            plugin, "register_plugin_action", _register_action
        ):
            instance._install_actions()

        self.assertIsNotNone(instance._diagnostics_action)
        self.assertIsNotNone(instance._refresh_action)
        self.assertIsNotNone(instance._panel_action)

        self.assertEqual(len(registered), 3)
        self.assertEqual(
            [item[0].text for item in registered],
            [
                "Runtime Diagnostics",
                "Refresh Catalog + Help",
                "Show Whitebox Panel",
            ],
        )
        self.assertTrue(all(item[1] == instance._menu_label for item in registered))

        diagnostics_cb = instance._diagnostics_action.triggered.callback
        refresh_cb = instance._refresh_action.triggered.callback
        panel_cb = instance._panel_action.triggered.callback
        self.assertIsNotNone(diagnostics_cb)
        self.assertIsNotNone(refresh_cb)
        self.assertIsNotNone(panel_cb)
        self.assertEqual(diagnostics_cb.__func__.__name__, "_show_diagnostics")
        self.assertEqual(refresh_cb.__func__.__name__, "_refresh_catalog")
        self.assertEqual(panel_cb.__func__.__name__, "_toggle_panel")

    def test_init_gui_runs_install_and_refresh_on_supported_host(self):
        iface = _FakeIface()
        instance = plugin.WhiteboxWorkflowsPlugin(iface)

        calls = []

        with patch.object(plugin, "qgis_major_version", lambda: 4), patch.object(
            plugin, "register_provider", lambda _iface, _provider: True
        ), patch.object(
            instance,
            "_install_panel",
            lambda *_a, **_k: calls.append("install_panel"),
        ), patch.object(
            instance,
            "_install_actions",
            lambda *_a, **_k: calls.append("install_actions"),
        ), patch.object(
            instance,
            "_refresh_catalog",
            lambda *_a, **kwargs: calls.append(("refresh_catalog", kwargs.get("silent"))),
        ):
            instance.initGui()

        self.assertTrue(instance._provider_registered)
        self.assertEqual(
            calls,
            [
                "install_panel",
                "install_actions",
                ("refresh_catalog", True),
            ],
        )

    def test_init_gui_noops_on_unsupported_qgis_major(self):
        iface = _FakeIface()
        instance = plugin.WhiteboxWorkflowsPlugin(iface)

        provider_calls = []

        with patch.object(plugin, "qgis_major_version", lambda: 3), patch.object(
            plugin,
            "register_provider",
            lambda _iface, _provider: provider_calls.append("register") or True,
        ), patch.object(
            instance,
            "_install_panel",
            lambda *_a, **_k: provider_calls.append("install_panel"),
        ), patch.object(
            instance,
            "_install_actions",
            lambda *_a, **_k: provider_calls.append("install_actions"),
        ), patch.object(
            instance,
            "_refresh_catalog",
            lambda *_a, **_k: provider_calls.append("refresh_catalog"),
        ):
            instance.initGui()

        self.assertFalse(instance._provider_registered)
        self.assertEqual(provider_calls, [])

    def test_init_gui_stops_when_provider_registration_fails(self):
        iface = _FakeIface()
        instance = plugin.WhiteboxWorkflowsPlugin(iface)

        setup_calls = []

        with patch.object(plugin, "qgis_major_version", lambda: 4), patch.object(
            plugin, "register_provider", lambda _iface, _provider: False
        ), patch.object(
            instance,
            "_install_panel",
            lambda *_a, **_k: setup_calls.append("install_panel"),
        ), patch.object(
            instance,
            "_install_actions",
            lambda *_a, **_k: setup_calls.append("install_actions"),
        ), patch.object(
            instance,
            "_refresh_catalog",
            lambda *_a, **_k: setup_calls.append("refresh_catalog"),
        ):
            instance.initGui()

        self.assertFalse(instance._provider_registered)
        self.assertEqual(setup_calls, [])

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

    def test_show_diagnostics_falls_back_to_message_bar_when_dialog_fails(self):
        class _IfaceWithWarningBar(_FakeIface):
            def __init__(self):
                self.warning_calls = []

            class _Bar:
                def __init__(self, sink):
                    self._sink = sink

                def pushWarning(self, title, text):
                    self._sink.append((title, text))

            def messageBar(self):
                return self._Bar(self.warning_calls)

        iface = _IfaceWithWarningBar()
        instance = plugin.WhiteboxWorkflowsPlugin(iface)

        with patch.object(
            plugin,
            "gather_runtime_diagnostics",
            lambda **_kwargs: {"status": "error", "error": "runtime not ready"},
        ), patch.object(
            plugin,
            "diagnostics_text",
            lambda _payload: "diagnostics summary",
        ), patch.object(
            plugin.QMessageBox,
            "information",
            lambda *_args, **_kwargs: (_ for _ in ()).throw(RuntimeError("dialog unavailable")),
        ):
            instance._show_diagnostics()

        self.assertEqual(
            iface.warning_calls,
            [("Whitebox Workflows", "Diagnostics unavailable as dialog; see logs.")],
        )

    def test_open_tool_from_panel_records_recent_and_notifies_on_success(self):
        iface = _FakeIface()
        instance = plugin.WhiteboxWorkflowsPlugin(iface)

        recorded_last = []
        recorded_recent = []
        info_calls = []

        instance._record_last_tool = lambda tool_id: recorded_last.append(tool_id)
        instance._record_recent_tool = lambda tool_id: recorded_recent.append(tool_id)
        instance._notify_info = lambda message: info_calls.append(message)

        with patch.object(plugin, "open_processing_algorithm_dialog", lambda *_args, **_kwargs: True):
            instance._open_tool_from_panel("d8_pointer")

        self.assertEqual(recorded_last, ["d8_pointer"])
        self.assertEqual(recorded_recent, ["d8_pointer"])
        self.assertEqual(info_calls, ["Opening tool: d8_pointer"])

    def test_open_tool_from_panel_warns_when_host_dialog_unavailable(self):
        iface = _FakeIface()
        instance = plugin.WhiteboxWorkflowsPlugin(iface)

        recorded_last = []
        recorded_recent = []
        warning_calls = []

        instance._record_last_tool = lambda tool_id: recorded_last.append(tool_id)
        instance._record_recent_tool = lambda tool_id: recorded_recent.append(tool_id)
        instance._notify_warning = lambda message: warning_calls.append(message)

        with patch.object(plugin, "open_processing_algorithm_dialog", lambda *_args, **_kwargs: False):
            instance._open_tool_from_panel("d8_pointer")

        self.assertEqual(recorded_last, [])
        self.assertEqual(recorded_recent, [])
        self.assertEqual(
            warning_calls,
            [
                "Unable to open dialog for d8_pointer; host processing API not available.",
            ],
        )

    def test_unload_is_idempotent_and_clears_registered_references(self):
        iface = _FakeIface()
        instance = plugin.WhiteboxWorkflowsPlugin(iface)

        panel_obj = _FakePanel()
        panel_action = object()
        refresh_action = object()
        diagnostics_action = object()

        instance._dock_panel = panel_obj
        instance._panel_action = panel_action
        instance._refresh_action = refresh_action
        instance._diagnostics_action = diagnostics_action
        instance._provider_registered = True

        dock_calls = []
        action_calls = []
        provider_calls = []

        with patch.object(
            plugin,
            "unregister_dock_widget",
            lambda _iface, panel: dock_calls.append(panel) or True,
        ), patch.object(
            plugin,
            "unregister_plugin_action",
            lambda _iface, action, menu: action_calls.append((action, menu)) or True,
        ), patch.object(
            plugin,
            "unregister_provider",
            lambda _iface, _provider: provider_calls.append("provider") or True,
        ):
            instance.unload()

            self.assertIsNone(instance._dock_panel)
            self.assertIsNone(instance._panel_action)
            self.assertIsNone(instance._refresh_action)
            self.assertIsNone(instance._diagnostics_action)
            self.assertFalse(instance._provider_registered)

            self.assertEqual(dock_calls, [panel_obj])
            self.assertEqual(
                action_calls,
                [
                    (panel_action, instance._menu_label),
                    (refresh_action, instance._menu_label),
                    (diagnostics_action, instance._menu_label),
                ],
            )
            self.assertEqual(provider_calls, ["provider"])

            instance.unload()

        self.assertEqual(dock_calls, [panel_obj])
        self.assertEqual(
            action_calls,
            [
                (panel_action, instance._menu_label),
                (refresh_action, instance._menu_label),
                (diagnostics_action, instance._menu_label),
            ],
        )
        self.assertEqual(provider_calls, ["provider"])


if __name__ == "__main__":
    unittest.main()
