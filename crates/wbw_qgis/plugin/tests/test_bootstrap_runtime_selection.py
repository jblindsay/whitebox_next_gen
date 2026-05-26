import os
import pathlib
import sys
import tempfile
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
        ), patch.object(bootstrap, "_discover_external_python_candidates", return_value=[]):
            session = bootstrap.create_runtime_session(include_pro=True, tier="open")

        self.assertIsInstance(session, DummySession)
        self.assertFalse(session.include_pro)

    def test_rewrites_windows_qgis_launcher_to_embedded_python(self):
        with tempfile.TemporaryDirectory() as tmp_dir:
            install_root = os.path.join(tmp_dir, "QGIS")
            bin_dir = os.path.join(install_root, "bin")
            apps_python_dir = os.path.join(install_root, "apps", "Python312")
            os.makedirs(bin_dir, exist_ok=True)
            os.makedirs(apps_python_dir, exist_ok=True)

            launcher = os.path.join(bin_dir, "python3.exe")
            embedded = os.path.join(apps_python_dir, "python.exe")

            for file_path in (launcher, embedded):
                with open(file_path, "w", encoding="utf-8") as handle:
                    handle.write("")
                os.chmod(file_path, 0o755)

            with patch.object(bootstrap.os, "name", "nt"), patch.object(bootstrap, "Path", pathlib.PosixPath):
                rewritten = bootstrap._rewrite_windows_qgis_launcher(launcher)

            self.assertEqual(rewritten, embedded)

    def test_discovery_skips_windows_qgis_launcher_stub_when_embedded_python_exists(self):
        with tempfile.TemporaryDirectory() as tmp_dir:
            install_root = os.path.join(tmp_dir, "QGIS")
            bin_dir = os.path.join(install_root, "bin")
            apps_python_dir = os.path.join(install_root, "apps", "Python312")
            os.makedirs(bin_dir, exist_ok=True)
            os.makedirs(apps_python_dir, exist_ok=True)

            launcher = os.path.join(bin_dir, "python3.exe")
            embedded = os.path.join(apps_python_dir, "python.exe")

            for file_path in (launcher, embedded):
                with open(file_path, "w", encoding="utf-8") as handle:
                    handle.write("")
                os.chmod(file_path, 0o755)

            with patch.object(bootstrap.os, "name", "nt"), patch.object(bootstrap, "Path", pathlib.PosixPath), patch.dict(
                bootstrap.os.environ, {}, clear=True
            ), patch.object(bootstrap, "get_runtime_preferences", return_value=("auto", "")), patch.object(
                bootstrap.shutil, "which", return_value=launcher
            ):
                candidates = bootstrap._discover_external_python_candidates()

            self.assertIn(embedded, candidates)
            self.assertNotIn(launcher, candidates)


if __name__ == "__main__":
    unittest.main()