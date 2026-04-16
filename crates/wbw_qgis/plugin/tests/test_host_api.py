import os
import sys
import unittest


_PLUGIN_ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), ".."))
if _PLUGIN_ROOT not in sys.path:
    sys.path.insert(0, _PLUGIN_ROOT)

from whitebox_workflows_qgis import host_api  # noqa: E402


class _FakeMenu:
    def __init__(self):
        self.added = []
        self.removed = []

    def addAction(self, action):
        self.added.append(action)

    def removeAction(self, action):
        self.removed.append(action)


class _FakeMainWindow:
    def __init__(self):
        self.added = []
        self.removed = []

    def addAction(self, action):
        self.added.append(action)

    def removeAction(self, action):
        self.removed.append(action)


class _PrimaryIface:
    def __init__(self):
        self.menu_calls = []
        self.toolbar_calls = []
        self.remove_menu_calls = []
        self.remove_toolbar_calls = []
        self._plugin_menu = _FakeMenu()
        self._main_window = _FakeMainWindow()

    def addPluginToMenu(self, menu_label, action):
        self.menu_calls.append((menu_label, action))

    def addToolBarIcon(self, action):
        self.toolbar_calls.append(action)

    def removePluginMenu(self, menu_label, action):
        self.remove_menu_calls.append((menu_label, action))

    def removeToolBarIcon(self, action):
        self.remove_toolbar_calls.append(action)

    def pluginMenu(self):
        return self._plugin_menu

    def mainWindow(self):
        return self._main_window


class _FallbackIface:
    def __init__(self):
        self._plugin_menu = _FakeMenu()
        self._main_window = _FakeMainWindow()

    def pluginMenu(self):
        return self._plugin_menu

    def mainWindow(self):
        return self._main_window


class HostApiPluginActionTests(unittest.TestCase):
    def test_register_plugin_action_uses_primary_menu_without_duplicate_fallbacks(self):
        iface = _PrimaryIface()
        action = object()

        registered = host_api.register_plugin_action(iface, action, "&Whitebox Workflows")

        self.assertTrue(registered)
        self.assertEqual(iface.menu_calls, [("&Whitebox Workflows", action)])
        self.assertEqual(iface.toolbar_calls, [action])
        self.assertEqual(iface._plugin_menu.added, [])
        self.assertEqual(iface._main_window.added, [])

    def test_register_plugin_action_uses_plugin_menu_fallback_when_primary_api_missing(self):
        iface = _FallbackIface()
        action = object()

        registered = host_api.register_plugin_action(iface, action, "&Whitebox Workflows")

        self.assertTrue(registered)
        self.assertEqual(iface._plugin_menu.added, [action])
        self.assertEqual(iface._main_window.added, [])

    def test_unregister_plugin_action_removes_from_all_supported_hosts(self):
        iface = _PrimaryIface()
        action = object()

        removed = host_api.unregister_plugin_action(iface, action, "&Whitebox Workflows")

        self.assertTrue(removed)
        self.assertEqual(iface.remove_menu_calls, [("&Whitebox Workflows", action)])
        self.assertEqual(iface.remove_toolbar_calls, [action])
        self.assertEqual(iface._plugin_menu.removed, [action])
        self.assertEqual(iface._main_window.removed, [action])


if __name__ == "__main__":
    unittest.main()