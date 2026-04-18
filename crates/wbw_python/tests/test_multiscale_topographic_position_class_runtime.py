import inspect
import unittest

import whitebox_workflows as wbw


class MultiscaleTopographicPositionClassRuntimeTests(unittest.TestCase):
    def test_environment_exposes_method(self):
        env = wbw.WbEnvironment()
        self.assertTrue(hasattr(env, "multiscale_topographic_position_class"))
        signature = inspect.signature(env.multiscale_topographic_position_class)
        self.assertIn("local_min_scale", str(signature))
        self.assertIn("output_confidence_path", str(signature))

    def test_tool_is_present_in_catalog(self):
        env = wbw.WbEnvironment()
        self.assertIn("multiscale_topographic_position_class", env.list_tools())


if __name__ == "__main__":
    unittest.main()