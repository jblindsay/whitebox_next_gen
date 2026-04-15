import os
import sys
import unittest


_PLUGIN_ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), ".."))
if _PLUGIN_ROOT not in sys.path:
    sys.path.insert(0, _PLUGIN_ROOT)

from whitebox_workflows_qgis import panel  # noqa: E402


class _FakeEvent:
    def __init__(self, event_type, key_value=None):
        self._event_type = event_type
        self._key_value = key_value

    def type(self):
        return self._event_type

    def key(self):
        return self._key_value


class PanelInteractionTests(unittest.TestCase):
    def setUp(self):
        self.dock = panel.WhiteboxDockPanel(parent=None)

    def test_trigger_diagnostics_invokes_callback(self):
        calls = []

        self.dock.on_diagnostics(lambda: calls.append("called"))
        self.dock._trigger_diagnostics()

        self.assertEqual(calls, ["called"])

    def test_session_banner_mouse_click_opens_diagnostics(self):
        calls = []
        self.dock.on_session_banner_clicked(lambda: calls.append("clicked"))

        handled = self.dock.eventFilter(
            self.dock._session_banner_label,
            _FakeEvent(panel.QEvent.MouseButtonRelease),
        )

        self.assertTrue(handled)
        self.assertEqual(calls, ["clicked"])

    def test_session_banner_enter_key_opens_diagnostics(self):
        calls = []
        self.dock.on_session_banner_clicked(lambda: calls.append("enter"))

        handled = self.dock.eventFilter(
            self.dock._session_banner_label,
            _FakeEvent(panel.QEvent.KeyPress, panel.Qt.Key_Return),
        )

        self.assertTrue(handled)
        self.assertEqual(calls, ["enter"])

    def test_session_banner_space_key_opens_diagnostics(self):
        calls = []
        self.dock.on_session_banner_clicked(lambda: calls.append("space"))

        handled = self.dock.eventFilter(
            self.dock._session_banner_label,
            _FakeEvent(panel.QEvent.KeyPress, panel.Qt.Key_Space),
        )

        self.assertTrue(handled)
        self.assertEqual(calls, ["space"])

    def test_session_banner_other_key_does_not_open_diagnostics(self):
        calls = []
        self.dock.on_session_banner_clicked(lambda: calls.append("bad"))

        handled = self.dock.eventFilter(
            self.dock._session_banner_label,
            _FakeEvent(panel.QEvent.KeyPress, -999),
        )

        self.assertFalse(handled)
        self.assertEqual(calls, [])

    def test_hidden_by_default_tools_are_filtered_without_query(self):
        self.dock.set_catalog(
            [
                {
                    "id": "visible_tool",
                    "display_name": "Visible Tool",
                    "category": "Hydrology",
                    "summary": "Always visible",
                    "locked": False,
                    "display_default_visible": True,
                },
                {
                    "id": "hidden_tool",
                    "display_name": "Hidden Tool",
                    "category": "Hydrology",
                    "summary": "Hidden until searched",
                    "locked": False,
                    "display_default_visible": False,
                },
            ]
        )

        self.dock._refresh_results("")
        self.assertEqual(self.dock._filtered_tool_ids, ["visible_tool"])

        self.dock._refresh_results("hidden")
        self.assertEqual(self.dock._filtered_tool_ids, ["hidden_tool"])


if __name__ == "__main__":
    unittest.main()
