import os
import sys
import unittest


_PLUGIN_ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), ".."))
if _PLUGIN_ROOT not in sys.path:
    sys.path.insert(0, _PLUGIN_ROOT)

from whitebox_workflows_qgis.plugin import WhiteboxWorkflowsPlugin  # noqa: E402


class _DummyIface:
    def mainWindow(self):
        return None


class PluginRuntimeWarningTests(unittest.TestCase):
    def test_downgraded_runtime_warning_takes_precedence_over_compiled_without_pro(self):
        plugin = WhiteboxWorkflowsPlugin(_DummyIface())
        plugin.provider.include_pro = True

        warning = plugin._build_pro_downgrade_warning(
            {
                "compiled_with_pro_support": False,
                "include_pro": False,
            }
        )

        self.assertIn("runtime downgraded to open mode", warning)
        self.assertIn("Open tools remain available", warning)
        self.assertNotIn("Reinstall wbw_python with --pro", warning)


if __name__ == "__main__":
    unittest.main()