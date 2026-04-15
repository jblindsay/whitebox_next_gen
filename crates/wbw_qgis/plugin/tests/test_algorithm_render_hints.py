import os
import sys
import unittest


_PLUGIN_ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), ".."))
if _PLUGIN_ROOT not in sys.path:
    sys.path.insert(0, _PLUGIN_ROOT)

from whitebox_workflows_qgis import algorithm  # noqa: E402


class AlgorithmRenderHintTests(unittest.TestCase):
    def test_resolve_render_hint_for_named_output(self):
        hints = {"output": "categorical", "raster": "continuous"}
        value = "/tmp/result.tif"

        resolved = algorithm._resolve_render_hint_for_output(hints, "output", value)

        self.assertEqual(resolved, "categorical")

    def test_resolve_render_hint_for_raster_fallback_key(self):
        hints = {"raster": "categorical"}
        value = "/tmp/result.tif"

        resolved = algorithm._resolve_render_hint_for_output(hints, "unmatched_key", value)

        self.assertEqual(resolved, "categorical")

    def test_resolve_render_hint_for_default_raster(self):
        hints = {"default_raster": "categorical"}
        value = "/tmp/result.tif"

        resolved = algorithm._resolve_render_hint_for_output(hints, "unmatched_key", value)

        self.assertEqual(resolved, "categorical")

    def test_resolve_render_hint_returns_empty_for_non_raster_without_key(self):
        hints = {"default_raster": "categorical"}
        value = "/tmp/report.html"

        resolved = algorithm._resolve_render_hint_for_output(hints, "unmatched_key", value)

        self.assertEqual(resolved, "")


if __name__ == "__main__":
    unittest.main()
