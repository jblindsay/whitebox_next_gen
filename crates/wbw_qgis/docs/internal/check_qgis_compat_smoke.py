#!/usr/bin/env python3
"""Lightweight non-host smoke checks for QGIS 3/4 compatibility scaffolding.

This script is intentionally host-agnostic: it does not require launching QGIS.
It validates that compatibility-critical plugin modules import and that key adapter
functions are present and callable at import-time.
"""

from __future__ import annotations

import importlib
import sys
from typing import Iterable

MODULES: tuple[str, ...] = (
    "whitebox_workflows_qgis.host_api",
    "whitebox_workflows_qgis.settings",
    "whitebox_workflows_qgis.field_calculator_dialog",
    "whitebox_workflows_qgis.panel",
    "whitebox_workflows_qgis.plugin",
)

HOST_API_REQUIRED_CALLABLES: tuple[str, ...] = (
    "host_capabilities",
    "is_supported_host",
    "is_supported_host_major",
    "qgis_major_version",
    "qgis_version_string",
    "run_dialog",
    "run_menu",
    "open_local_file",
    "resolve_qevent_type",
    "resolve_qt_constant",
    "push_host_message",
    "show_info_dialog",
)


def _import_modules(names: Iterable[str]) -> list[tuple[str, bool, str]]:
    results: list[tuple[str, bool, str]] = []
    for name in names:
        try:
            importlib.import_module(name)
            results.append((name, True, ""))
        except Exception as exc:  # pragma: no cover
            results.append((name, False, f"{type(exc).__name__}: {exc}"))
    return results


def _check_host_api() -> list[tuple[str, bool, str]]:
    results: list[tuple[str, bool, str]] = []
    try:
        host_api = importlib.import_module("whitebox_workflows_qgis.host_api")
    except Exception as exc:  # pragma: no cover
        return [("host_api_import", False, f"{type(exc).__name__}: {exc}")]

    for symbol in HOST_API_REQUIRED_CALLABLES:
        value = getattr(host_api, symbol, None)
        if callable(value):
            results.append((f"host_api.{symbol}", True, ""))
        else:
            results.append((f"host_api.{symbol}", False, "missing or not callable"))

    caps = getattr(host_api, "host_capabilities", None)
    if callable(caps):
        try:
            # Some adapter versions require an iface argument; pass None for
            # non-host smoke checks.
            payload = caps(None)
            is_mapping = isinstance(payload, dict)
            has_keys = is_mapping and all(
                key in payload for key in ("major", "supported_major", "missing_required")
            )
            if has_keys:
                results.append(("host_api.host_capabilities_payload", True, ""))
            else:
                results.append(("host_api.host_capabilities_payload", False, "missing expected keys"))
        except Exception as exc:  # pragma: no cover
            results.append(("host_api.host_capabilities_payload", False, f"call failed: {exc}"))

    return results


def main() -> int:
    checks: list[tuple[str, bool, str]] = []
    checks.extend(_import_modules(MODULES))
    checks.extend(_check_host_api())

    failed = [c for c in checks if not c[1]]

    for name, ok, details in checks:
        status = "PASS" if ok else "FAIL"
        if details:
            print(f"[{status}] {name} :: {details}")
        else:
            print(f"[{status}] {name}")

    if failed:
        print(f"\nResult: FAILED ({len(failed)} checks failed)")
        return 1

    print(f"\nResult: PASSED ({len(checks)} checks)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
