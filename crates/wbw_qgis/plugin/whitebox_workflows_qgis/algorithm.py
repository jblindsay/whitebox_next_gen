from __future__ import annotations

import json
import re
from typing import Any

from .bootstrap import create_runtime_session
from .help import get_help_url

try:
    from qgis.core import (
        QgsProcessing,
        QgsProcessingAlgorithm,
        QgsProcessingException,
        QgsProcessingParameterBoolean,
        QgsProcessingParameterFile,
        QgsProcessingParameterFileDestination,
        QgsProcessingParameterNumber,
        QgsProcessingParameterRasterLayer,
        QgsProcessingParameterString,
        QgsProcessingParameterVectorLayer,
    )
except ImportError:  # pragma: no cover
    class QgsProcessingAlgorithm:  # type: ignore[override]
        def addParameter(self, *_args, **_kwargs):
            return None

    class QgsProcessingException(Exception):
        pass

    class QgsProcessing:  # type: ignore[override]
        TypeVectorAnyGeometry = 0

    class _Dummy:  # type: ignore[override]
        def __init__(self, *_args, **_kwargs):
            pass

    QgsProcessingParameterBoolean = _Dummy
    QgsProcessingParameterFile = _Dummy
    QgsProcessingParameterFileDestination = _Dummy
    QgsProcessingParameterNumber = _Dummy
    QgsProcessingParameterRasterLayer = _Dummy
    QgsProcessingParameterString = _Dummy
    QgsProcessingParameterVectorLayer = _Dummy


def _normalize_group_id(text: str) -> str:
    normalized = re.sub(r"[^a-z0-9]+", "_", text.lower()).strip("_")
    return normalized or "general"


def _looks_like_output(name: str, description: str) -> bool:
    text = f"{name} {description}".lower()
    markers = (
        "output",
        "result",
        "report",
        "summary",
        "export",
        "destination",
        "save",
        "write",
    )
    return any(m in text for m in markers)


def _infer_kind(name: str, description: str) -> str:
    n = name.lower()
    d = description.lower()

    if _looks_like_output(n, d):
        return "file_out"

    if any(tok in n or tok in d for tok in ("raster", "dem", "grid", "geotiff", "tif")):
        return "raster_in"

    if any(tok in n or tok in d for tok in ("vector", "shp", "geojson", "geopackage", "features")):
        return "vector_in"

    if any(tok in n or tok in d for tok in ("lidar", "las", "laz", "zlidar")):
        return "file_in"

    if n.startswith(("is_", "has_", "use_", "enable_", "auto_")) or "true/false" in d:
        return "bool"

    if any(tok in n or tok in d for tok in ("integer", " int ", "count", "iterations", "num_")):
        return "int"

    if any(tok in n or tok in d for tok in ("distance", "threshold", "factor", "percent", "ratio", "angle", "weight", "radius")):
        return "double"

    if any(tok in n or tok in d for tok in ("file", "path", "directory", "folder", "csv", "json", "html")):
        return "file_in"

    return "string"


def _is_raster_path(path: str) -> bool:
    p = str(path).lower()
    return p.endswith((".tif", ".tiff", ".img", ".bil", ".flt", ".sdat", ".rdc"))


def _coerce_render_hints(raw: Any) -> dict[str, str]:
    if not isinstance(raw, dict):
        return {}
    out: dict[str, str] = {}
    for k, v in raw.items():
        key = str(k).strip()
        value = str(v).strip()
        if key and value:
            out[key] = value
    return out


def _render_hint_summary(hints: dict[str, str]) -> str:
    if not hints:
        return ""
    pieces = [f"{k}={v}" for k, v in sorted(hints.items())]
    return "Render hints: " + ", ".join(pieces)


class WhiteboxCatalogAlgorithm(QgsProcessingAlgorithm):
    def __init__(self, provider, manifest: dict[str, Any]):
        super().__init__()
        self._provider = provider
        self._manifest = manifest
        self._param_kinds: list[tuple[str, str, bool]] = []
        self._render_hints = _coerce_render_hints(self._manifest.get("render_hints", {}))

    def createInstance(self):
        return WhiteboxCatalogAlgorithm(self._provider, dict(self._manifest))

    def name(self):
        return self._manifest.get("id", "")

    def displayName(self):
        title = self._manifest.get("display_name", self.name())
        if bool(self._manifest.get("locked", False)):
            return f"[Locked] {title}"
        return title

    def group(self):
        return str(self._manifest.get("category", "General"))

    def groupId(self):
        return _normalize_group_id(self.group())

    def shortHelpString(self):
        summary = self._manifest.get("summary", "")
        hint_text = _render_hint_summary(self._render_hints)
        if bool(self._manifest.get("locked", False)):
            reason = self._manifest.get("locked_reason", "license_tier_insufficient")
            return (
                f"{summary}\n\n"
                "This tool is visible in the catalog but locked for the current runtime tier.\n"
                f"Reason: {reason}."
                + (f"\n\n{hint_text}" if hint_text else "")
            )
        if hint_text:
            return f"{summary}\n\n{hint_text}"
        return summary

    def helpUrl(self):
        path = self._provider.help_path_for_tool(self.name()) if self._provider else ""
        if path:
            return path
        return get_help_url(self.name())

    def initAlgorithm(self, _config=None):
        self._param_kinds = []
        for p in self._manifest.get("params", []):
            name = p.get("name", "")
            if not name:
                continue
            description = p.get("description", name)
            required = bool(p.get("required", False))
            kind = _infer_kind(name, description)

            if kind == "raster_in":
                qgs_param = QgsProcessingParameterRasterLayer(
                    name,
                    description,
                    defaultValue=None,
                    optional=not required,
                )
            elif kind == "vector_in":
                qgs_param = QgsProcessingParameterVectorLayer(
                    name,
                    description,
                    [QgsProcessing.TypeVectorAnyGeometry],
                    defaultValue=None,
                    optional=not required,
                )
            elif kind == "bool":
                qgs_param = QgsProcessingParameterBoolean(
                    name,
                    description,
                    defaultValue=False,
                    optional=not required,
                )
            elif kind == "int":
                qgs_param = QgsProcessingParameterNumber(
                    name,
                    description,
                    QgsProcessingParameterNumber.Integer,
                    defaultValue=0,
                    optional=not required,
                )
            elif kind == "double":
                qgs_param = QgsProcessingParameterNumber(
                    name,
                    description,
                    QgsProcessingParameterNumber.Double,
                    defaultValue=0.0,
                    optional=not required,
                )
            elif kind == "file_out":
                qgs_param = QgsProcessingParameterFileDestination(
                    name,
                    description,
                    defaultValue=None,
                    optional=not required,
                )
            elif kind == "file_in":
                qgs_param = QgsProcessingParameterFile(
                    name,
                    description,
                    behavior=QgsProcessingParameterFile.File,
                    defaultValue=None,
                    optional=not required,
                )
            else:
                qgs_param = QgsProcessingParameterString(
                    name,
                    description,
                    defaultValue="",
                    optional=not required,
                )

            self.addParameter(qgs_param)
            self._param_kinds.append((name, kind, required))

    def processAlgorithm(self, parameters, context, feedback):
        if bool(self._manifest.get("locked", False)):
            reason = self._manifest.get("locked_reason", "license_tier_insufficient")
            raise QgsProcessingException(
                "This tool is locked for the active runtime tier. "
                f"Reason: {reason}."
            )

        args: dict[str, Any] = {}

        for name, kind, required in self._param_kinds:
            if kind == "raster_in":
                lyr = self.parameterAsRasterLayer(parameters, name, context)
                if lyr is None:
                    if required:
                        raise QgsProcessingException(f"Missing required raster input: {name}")
                    continue
                args[name] = lyr.source()
            elif kind == "vector_in":
                lyr = self.parameterAsVectorLayer(parameters, name, context)
                if lyr is None:
                    if required:
                        raise QgsProcessingException(f"Missing required vector input: {name}")
                    continue
                args[name] = lyr.source()
            elif kind == "bool":
                args[name] = bool(self.parameterAsBool(parameters, name, context))
            elif kind == "int":
                args[name] = int(self.parameterAsInt(parameters, name, context))
            elif kind == "double":
                args[name] = float(self.parameterAsDouble(parameters, name, context))
            else:
                value = self.parameterAsString(parameters, name, context)
                if not value:
                    if required:
                        raise QgsProcessingException(f"Missing required parameter: {name}")
                    continue
                args[name] = value

        session = create_runtime_session(
            include_pro=self._provider.include_pro,
            tier=self._provider.tier,
        )

        def _stream_callback(event):
            try:
                payload = json.loads(event) if isinstance(event, str) else event
            except Exception:
                return
            if not isinstance(payload, dict):
                return
            event_type = payload.get("type")
            if event_type == "progress":
                try:
                    feedback.setProgress(float(payload.get("percent", 0.0)))
                except Exception:
                    pass
            elif event_type == "message":
                msg = payload.get("message")
                if msg:
                    feedback.pushInfo(str(msg))

        response_raw = session.run_tool_json_stream(
            self.name(),
            json.dumps(args),
            _stream_callback,
        )

        response = (
            json.loads(response_raw)
            if isinstance(response_raw, str)
            else response_raw
        )
        if not isinstance(response, dict):
            return {}

        if feedback.isCanceled():
            raise QgsProcessingException("Processing cancelled.")

        outputs = response.get("outputs", response)
        result: dict[str, Any] = {}

        if isinstance(outputs, dict):
            for key, value in outputs.items():
                if isinstance(value, dict):
                    path = value.get("path")
                    if isinstance(path, str) and path:
                        result[key] = path
                elif isinstance(value, str):
                    result[key] = value

        # Ensure declared destination parameters are always returned when present.
        for name, kind, _required in self._param_kinds:
            if kind == "file_out" and name in args:
                result[name] = args[name]

        # Emit best-effort render hint messages for downstream display handling.
        for key, value in list(result.items()):
            if not isinstance(value, str) or not value:
                continue
            hint = self._render_hints.get(key) or self._render_hints.get("raster")
            if hint is None and _is_raster_path(value):
                hint = self._render_hints.get("default_raster")
            if hint:
                feedback.pushInfo(f"Render hint for {key}: {hint}")

        return result


def build_algorithms(provider, catalog: list[dict[str, Any]]) -> list[WhiteboxCatalogAlgorithm]:
    available = [m for m in catalog if not bool(m.get("locked", False))]
    locked = [m for m in catalog if bool(m.get("locked", False))]

    def _sort_key(item: dict[str, Any]) -> tuple[str, str, str]:
        return (
            str(item.get("category", "")),
            str(item.get("display_name", "")),
            str(item.get("id", "")),
        )

    manifests = sorted(available, key=_sort_key) + sorted(locked, key=_sort_key)
    return [WhiteboxCatalogAlgorithm(provider, m) for m in manifests]