import os
import sys
import unittest
from unittest.mock import patch


_PLUGIN_ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), ".."))
if _PLUGIN_ROOT not in sys.path:
    sys.path.insert(0, _PLUGIN_ROOT)

from whitebox_workflows_qgis import bootstrap  # noqa: E402


class BootstrapRuntimeSelectionTests(unittest.TestCase):
    def test_prefers_current_runtime_oss_downgrade_before_external_fallback(self):
        class DummySession:
            def __init__(self, include_pro, tier):
                if include_pro:
                    raise RuntimeError(
                        "include_pro=true requested but this build does not include Pro support"
                    )
                self.include_pro = include_pro
                self.tier = tier

            def get_runtime_capabilities_json(self):
                return (
                    '{"runtime_mode":"tier","effective_tier":"open",'
                    '"requested_tier":"open"}'
                )

        class DummyModule:
            RuntimeSession = DummySession

        with patch.object(bootstrap, "load_whitebox_workflows", return_value=DummyModule), patch.object(
            bootstrap, "ExternalRuntimeSession", side_effect=AssertionError("external runtime should not be used")
        ):
            session = bootstrap.create_runtime_session(include_pro=True, tier="open")

        self.assertIsInstance(session, DummySession)
        self.assertFalse(session.include_pro)


if __name__ == "__main__":
    unittest.main()