from __future__ import annotations

import importlib
import json


class RuntimeBootstrapError(RuntimeError):
    pass


def load_whitebox_workflows():
    try:
        return importlib.import_module("whitebox_workflows")
    except ImportError as exc:
        raise RuntimeBootstrapError(
            "The whitebox_workflows package is not available in this Python environment. "
            "Install or activate a QGIS-compatible WbW-Py build before loading the plugin."
        ) from exc


def create_runtime_session(include_pro: bool = True, tier: str = "open"):
    wbw = load_whitebox_workflows()
    return wbw.RuntimeSession(include_pro=include_pro, tier=tier)


def get_runtime_capabilities(include_pro: bool = True, tier: str = "open") -> dict:
    session = create_runtime_session(include_pro=include_pro, tier=tier)
    return json.loads(session.get_runtime_capabilities_json())


def get_tool_catalog(include_pro: bool = True, tier: str = "open") -> list[dict]:
    session = create_runtime_session(include_pro=include_pro, tier=tier)
    return json.loads(session.list_tool_catalog_json())


def get_tool_metadata(tool_id: str, include_pro: bool = True, tier: str = "open") -> dict:
    session = create_runtime_session(include_pro=include_pro, tier=tier)
    return json.loads(session.get_tool_metadata_json(tool_id))