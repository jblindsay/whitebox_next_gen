import os
import sys
import unittest
from unittest.mock import patch


_PLUGIN_ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), ".."))
if _PLUGIN_ROOT not in sys.path:
    sys.path.insert(0, _PLUGIN_ROOT)

from whitebox_workflows_qgis import algorithm, discovery, help as help_mod  # noqa: E402


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

    def test_multiscale_topographic_position_palette_classes_build_fixed_palette(self):
        class DummyClass:
            def __init__(self, value, color, label):
                self.value = value
                self.color = color
                self.label = label

        class DummyRenderer:
            Class = DummyClass

        with patch.object(algorithm, "QgsPalettedRasterRenderer", DummyRenderer), patch.object(
            algorithm, "QColor", lambda value: value
        ):
            classes = algorithm._multiscale_topographic_position_palette_classes()

        self.assertIsNotNone(classes)
        self.assertEqual(len(classes), 9)
        self.assertEqual(classes[0].value, 0)
        self.assertEqual(classes[0].label, "Lowland hollow")
        self.assertEqual(classes[0].color, "#355f8d")
        self.assertEqual(classes[-1].value, 8)
        self.assertEqual(classes[-1].label, "Upland knoll")

    def test_multiscale_catalog_injects_categorical_render_hints(self):
        catalog = [{"id": "multiscale_topographic_position_class", "render_hints": {}}]

        out = discovery._inject_multiscale_topographic_position_class_render_hints(catalog)

        self.assertEqual(out[0]["render_hints"]["path"], "categorical_raster")
        self.assertEqual(out[0]["render_hints"]["output"], "categorical_raster")

    def test_multiscale_help_file_is_bundled(self):
        path = help_mod.get_bundled_help_path("multiscale_topographic_position_class")
        self.assertTrue(bool(path))
        self.assertTrue(os.path.isfile(path))


if __name__ == "__main__":
    unittest.main()
