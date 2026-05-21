from __future__ import annotations

import glob
import json
import math
import os
import re
import sqlite3
import tempfile
from typing import Any

try:
    from osgeo import gdal
except Exception:  # pragma: no cover
    gdal = None

from .bootstrap import RuntimeBootstrapError, create_runtime_session, run_projection_wrapper
from .help import get_help_url
from .help_provider import get_help_provider
from .descriptions_provider import get_descriptions_provider

try:
    from qgis.PyQt.QtCore import QSettings
    from qgis.PyQt.QtGui import QColor
    from qgis.core import (
        QgsPalettedRasterRenderer,
        QgsProcessing,
        QgsProcessingAlgorithm,
        QgsProcessingException,
        QgsProcessingParameterBoolean,
        QgsProcessingParameterEnum,
        QgsProcessingParameterFile,
        QgsProcessingParameterFileDestination,
        QgsProcessingParameterNumber,
        QgsProcessingParameterRasterLayer,
        QgsProcessingParameterRasterDestination,
        QgsProcessingParameterString,
        QgsProcessingParameterVectorLayer,
        QgsProcessingParameterVectorDestination,
        QgsProcessingLayerPostProcessorInterface,
        QgsProject,
        QgsRasterLayer,
        QgsVectorFileWriter,
    )
    # QGIS API compatibility: some versions expose a dedicated multiple-raster
    # parameter class, others use the generic multiple-layers parameter.
    try:
        from qgis.core import QgsProcessingParameterMultipleRasterLayers
    except ImportError:
        try:
            from qgis.core import QgsProcessingParameterMultipleLayers
            QgsProcessingParameterMultipleRasterLayers = QgsProcessingParameterMultipleLayers
        except ImportError:
            QgsProcessingParameterMultipleRasterLayers = None
except ImportError:  # pragma: no cover
    class QSettings:  # type: ignore[override]
        def value(self, *_args, **_kwargs):
            return ""

        def setValue(self, *_args, **_kwargs):
            return None

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
    QgsProcessingParameterEnum = _Dummy
    QgsProcessingParameterFile = _Dummy
    QgsProcessingParameterFileDestination = _Dummy
    QgsProcessingParameterMultipleRasterLayers = _Dummy
    QgsProcessingParameterNumber = _Dummy
    QgsProcessingParameterRasterLayer = _Dummy
    QgsProcessingParameterRasterDestination = _Dummy
    QgsProcessingParameterString = _Dummy
    QgsProcessingParameterVectorLayer = _Dummy
    QgsProcessingParameterVectorDestination = _Dummy
    QgsProcessingLayerPostProcessorInterface = _Dummy
    QgsProject = _Dummy
    QgsRasterLayer = _Dummy
    QgsVectorFileWriter = _Dummy
    QgsPalettedRasterRenderer = _Dummy
    QColor = _Dummy


_MULTISCALE_TOPOGRAPHIC_POSITION_CLASSES: list[tuple[int, str, str]] = [
    # Local class uses hue (hollow/mid/knoll); broad class uses tint (low/intermediate/upland).
    (0, "Lowland hollow", "#7A3E2E"),
    (1, "Lowland mid-position", "#8A6A2B"),
    (2, "Lowland knoll", "#4E6A3D"),
    (3, "Intermediate hollow", "#A35D49"),
    (4, "Intermediate mid-position", "#B59048"),
    (5, "Intermediate knoll", "#6F9259"),
    (6, "Upland hollow", "#C98A73"),
    (7, "Upland mid-position", "#D8BC79"),
    (8, "Upland knoll", "#97BE7F"),
]


class _MstpStylePostProcessor(QgsProcessingLayerPostProcessorInterface):
    """Applies the fixed MSTP categorical renderer after layer load."""

    _instance = None

    @classmethod
    def create(cls):
        cls._instance = cls()
        return cls._instance

    def postProcessLayer(self, layer, context, feedback):  # noqa: N802 - QGIS API name
        if layer is None:
            return
        provider_getter = getattr(layer, "dataProvider", None)
        provider = provider_getter() if callable(provider_getter) else None
        if provider is None:
            return

        classes = _multiscale_topographic_position_palette_classes()
        if not classes:
            return

        try:
            renderer = QgsPalettedRasterRenderer(provider, 1, classes)
            set_renderer = getattr(layer, "setRenderer", None)
            if callable(set_renderer):
                set_renderer(renderer)
            trigger_repaint = getattr(layer, "triggerRepaint", None)
            if callable(trigger_repaint):
                trigger_repaint()
        except Exception:
            return


def _normalize_group_id(text: str) -> str:
    normalized = re.sub(r"[^a-z0-9]+", "_", text.lower()).strip("_")
    return normalized or "general"


def _looks_like_output(name: str, description: str) -> bool:
    n = name.lower().strip()
    d = description.lower().strip()

    # Prefer explicit output-like parameter names.
    if n in {"output", "out", "output_file", "output_path", "destination", "dst"}:
        return True
    if n.startswith(("output_", "out_", "destination_")):
        return True

    # Description-only hints must indicate persisted artifacts, not semantic "output units" text.
    persist_markers = (
        "output file",
        "output path",
        "destination file",
        "destination path",
        "save to",
        "write to",
        "report file",
    )
    return any(m in d for m in persist_markers)


def _looks_like_filepath_default(value: Any) -> bool:
    if value is None or isinstance(value, (list, dict, tuple, set)):
        return False
    text = str(value).strip()
    if not text:
        return False

    lower = text.lower()
    if lower in {"none", "null", "auto", "default"}:
        return False

    # Absolute/relative path indicators.
    if re.match(r"^[a-zA-Z]:[\\/]", text):
        return True
    if text.startswith(("/", "./", "../", "~", "\\")):
        return True
    if "/" in text or "\\" in text:
        return True

    # Common geospatial and table/document file extensions used by tools.
    if re.search(
        r"\.(shp|gpkg|geojson|topojson|json|csv|txt|tif|tiff|img|bil|flt|sdat|rdc|las|laz|zlidar|copc|e57|ply|html|htm|xml|sqlite|dbf|shx|prj|vrt)$",
        lower,
    ):
        return True

    return False


def _extract_enum_options(name: str, description: str, default_value: Any) -> list[str]:
    n = str(name or "").lower()
    d = str(description or "")
    dl = d.lower()

    # Common unit option style used by many tools.
    if n == "units" or "units:" in dl or "unit:" in dl:
        unit_opts = [
            opt
            for opt in ("degrees", "radians", "percent", "slope_radians")
            if opt in dl
        ]
        if unit_opts:
            return unit_opts

    # Parse "options: a, b, c" / "values: a, b, c" patterns.
    marker_index = -1
    marker_len = 0
    for marker in ("options:", "option:", "values:", "allowed:", "choices:"):
        idx = dl.find(marker)
        if idx >= 0:
            marker_index = idx
            marker_len = len(marker)
            break

    raw_opts: list[str] = []
    if marker_index >= 0:
        tail = d[marker_index + marker_len :]
        stop_tokens = (".", ";", "(", "[")
        for tok in stop_tokens:
            pos = tail.find(tok)
            if pos >= 0:
                tail = tail[:pos]
                break
        raw_opts = [p.strip(" \t\n\r'\"`") for p in tail.split(",")]

    options = [opt for opt in raw_opts if opt]

    # Parenthesized comma-separated option list: "Basis type (a, b, c)."
    # Only fire when all candidates are short identifier-like tokens.
    if not options:
        paren_match = re.search(r'\(([a-z][a-z0-9_]+(?:,\s*[a-z][a-z0-9_]+)+)\)', dl)
        if paren_match:
            candidates = [s.strip() for s in paren_match.group(1).split(",")]
            if all(len(s) <= 32 and re.match(r'^[a-z][a-z0-9_]*$', s) for s in candidates):
                options = candidates

    # Colon-prefix list: "Label: a | b | c" or "Label: a, b, c".
    # Use the FIRST colon so trailing "Default: x." clauses are excluded from
    # the option slice rather than becoming the anchor point.
    if not options:
        colon_idx = dl.find(':')
        if colon_idx >= 0:
            after = dl[colon_idx + 1:].strip()
            # Strip trailing "default: xxx" or "default xxx" clauses.
            after = re.sub(r'[\.\s]*\bdefault\b[:\s]+\S+[\s.]*$', '', after).strip().rstrip('.')
            if '|' in after:
                parts = [p.strip() for p in after.split('|')]
            elif '/' in after and ',' not in after:
                # Slash-separated identifiers only when no commas present, to avoid
                # catching prose like "origin/destination nodes" (which have commas
                # in surrounding context). Require ≥3 parts to avoid "a/b" pairs.
                slash_parts = [p.strip() for p in after.split('/')]
                if len(slash_parts) >= 3:
                    parts = slash_parts
                else:
                    parts = []
            else:
                # Normalise " or " and commas into a uniform split.
                normalised = re.sub(r'\bor\b', ',', after)
                parts = [p.strip() for p in normalised.split(',')]
            parts = [p for p in parts if p]
            _ident = re.compile(r'^[a-z][a-z0-9_]*$')
            if len(parts) >= 2 and all(len(p) <= 40 and _ident.match(p) for p in parts):
                options = parts

    if not options:
        return []

    default_text = str(default_value).strip().lower() if default_value is not None else ""
    if default_text and default_text not in [o.lower() for o in options]:
        options.insert(0, default_text)

    # De-duplicate while preserving order.
    out: list[str] = []
    seen = set()
    for opt in options:
        key = opt.lower()
        if key in seen:
            continue
        seen.add(key)
        out.append(opt)
    return out


def _coerce_bool_default(value: Any, fallback: bool = False) -> bool:
    if value is None:
        return fallback
    if isinstance(value, bool):
        return value
    if isinstance(value, (int, float)):
        return bool(value)
    text = str(value).strip().lower()
    if text in {"1", "true", "yes", "y", "on"}:
        return True
    if text in {"0", "false", "no", "n", "off"}:
        return False
    return fallback


def _coerce_int_default(value: Any, fallback: int = 0) -> int:
    if value is None:
        return fallback
    if isinstance(value, bool):
        return int(value)
    if isinstance(value, int):
        return value
    if isinstance(value, float):
        return int(value)
    if isinstance(value, str):
        text = value.strip().lower()
        if not text:
            return fallback
        # Some manifests use type-like tokens as defaults (e.g. "float").
        if text in {"float", "double", "string", "bool", "boolean", "integer", "int"}:
            return fallback
        try:
            return int(text)
        except Exception:
            try:
                return int(float(text))
            except Exception:
                return fallback
    return fallback


def _coerce_float_default(value: Any, fallback: float = 0.0) -> float:
    if value is None:
        return fallback
    if isinstance(value, bool):
        return float(int(value))
    if isinstance(value, (int, float)):
        return float(value)
    if isinstance(value, str):
        text = value.strip().lower()
        if not text:
            return fallback
        if text in {"float", "double", "string", "bool", "boolean", "integer", "int"}:
            return fallback
        try:
            return float(text)
        except Exception:
            return fallback
    return fallback


def _coerce_string_default(value: Any, fallback: str = "") -> str:
    if value is None:
        return fallback
    # Lists/dicts should not be injected as a string default in parameter widgets.
    if isinstance(value, (list, dict, tuple, set)):
        return fallback
    return str(value)


def _coerce_arg_from_default_type(value: Any, default_value: Any) -> Any:
    """Coerce runtime args using manifest default types when available.

    This is a defensive normalization layer for manifests that omit explicit
    type metadata. Coercion is conservative: if a value cannot be safely parsed,
    the original value is preserved.
    """
    if isinstance(default_value, bool):
        if isinstance(value, bool):
            return value
        if isinstance(value, (int, float)):
            return bool(value)
        text = str(value).strip().lower()
        if text in {"1", "true", "yes", "y", "on"}:
            return True
        if text in {"0", "false", "no", "n", "off"}:
            return False
        return value

    if isinstance(default_value, int) and not isinstance(default_value, bool):
        if isinstance(value, int) and not isinstance(value, bool):
            return value
        if isinstance(value, bool):
            return int(value)
        if isinstance(value, float):
            return int(value)
        if isinstance(value, str):
            text = value.strip()
            if not text:
                return value
            try:
                return int(text)
            except Exception:
                try:
                    return int(float(text))
                except Exception:
                    return value
        return value

    if isinstance(default_value, float):
        if isinstance(value, (int, float)) and not isinstance(value, bool):
            return float(value)
        if isinstance(value, str):
            text = value.strip()
            if not text:
                return value
            try:
                return float(text)
            except Exception:
                return value
        return value

    return value


def _is_param_explicitly_set(parameters: Any, name: str) -> bool:
    """Best-effort check whether a QGIS processing parameter was explicitly set."""
    if parameters is None:
        return False

    sentinel = object()
    try:
        raw = parameters.get(name, sentinel)
    except Exception:
        raw = sentinel

    if raw is sentinel or raw is None:
        return False

    is_null = getattr(raw, "isNull", None)
    if callable(is_null):
        try:
            if bool(is_null()):
                return False
        except Exception:
            pass

    if isinstance(raw, str):
        text = raw.strip().lower()
        return text not in {"", "none", "null", "nan"}

    if isinstance(raw, (list, tuple, dict, set)):
        return len(raw) > 0

    return True


def _safe_json_loads(text: str, fallback: Any) -> Any:
    try:
        return json.loads(str(text or ""))
    except Exception:
        return fallback


def _sanitize_args_using_runtime_schema(
    tool_id: str,
    args: dict[str, Any],
    session,
    feedback=None,
) -> dict[str, Any]:
    """Sanitize argument payload using runtime metadata when available.

    This removes unknown parameters, normalizes optional textual nulls for
    numeric fields, and coerces values using runtime defaults where possible.
    """
    if not isinstance(args, dict) or not args:
        return args

    metadata_raw = None
    try:
        metadata_raw = session.get_tool_metadata_json(tool_id)
    except Exception:
        return args

    metadata = _safe_json_loads(metadata_raw, {})
    if not isinstance(metadata, dict):
        return args

    params = metadata.get("params")
    defaults = metadata.get("defaults")
    if not isinstance(params, list):
        params = []
    if not isinstance(defaults, dict):
        defaults = {}

    allowed_names: set[str] = set()
    required_names: set[str] = set()
    for p in params:
        if not isinstance(p, dict):
            continue
        name = str(p.get("name", "")).strip()
        if not name:
            continue
        allowed_names.add(name)
        if bool(p.get("required", False)):
            required_names.add(name)

    sanitized = dict(args)

    # Drop unknown keys if runtime published a concrete schema.
    if allowed_names:
        unknown = [k for k in list(sanitized.keys()) if k not in allowed_names]
        for k in unknown:
            sanitized.pop(k, None)
        if unknown and feedback is not None:
            push_warn = getattr(feedback, "pushWarning", None)
            if callable(push_warn):
                push_warn(
                    f"{tool_id}: dropped unsupported parameters from QGIS payload: {', '.join(sorted(unknown))}"
                )

    # Coerce values using runtime defaults and normalize optional textual nulls.
    for k in list(sanitized.keys()):
        v = sanitized.get(k)
        default_v = defaults.get(k)
        is_required = k in required_names

        if v is None and not is_required:
            sanitized.pop(k, None)
            continue

        if isinstance(v, str):
            t = v.strip()
            tl = t.lower()

            # Normalize QGIS provider URIs for vector-like file paths.
            if ("|" in t or "dbname=" in tl) and _looks_like_vector_file_path(_normalize_vector_input_source(t)):
                t = _normalize_vector_input_source(t)
                tl = t.lower()
                sanitized[k] = t
                v = t

            if not t and not is_required:
                sanitized.pop(k, None)
                continue

            # Treat textual null-like values as "unset" for optional params
            # regardless of default type (numeric, enum/string, bool, paths).
            if tl in {"none", "null", "nan"} and not is_required:
                sanitized.pop(k, None)
                continue

            # Optional numeric params often arrive as textual null markers.
            if isinstance(default_v, (int, float)) and tl in {"none", "null", "nan"} and not is_required:
                sanitized.pop(k, None)
                continue

        if k in defaults:
            coerced = _coerce_arg_from_default_type(sanitized.get(k), default_v)
            if isinstance(coerced, float) and not math.isfinite(coerced):
                if is_required:
                    continue
                sanitized.pop(k, None)
                continue
            sanitized[k] = coerced

    return sanitized


def _derive_group_name(manifest: dict[str, Any]) -> str:
    # Prefer the taxonomy-stamped category that discover_tool_catalog() writes
    # from the canonical tool_taxonomy.resolved.json.  This is the single
    # source of truth shared across all three frontends (Python, R, QGIS).
    category = str(manifest.get("category", "") or "").strip()
    if category:
        return category
    return "General"


def _infer_kind(name: str, description: str, default_value: Any = None) -> str:
    n = name.lower()
    d = description.lower()

    # Runtime manifests often omit explicit parameter types. When a default is
    # a concrete boolean, preserve that signal so we create a checkbox widget
    # and serialize true/false values instead of string literals.
    if isinstance(default_value, bool):
        return "bool"
    if isinstance(default_value, str) and default_value.strip().lower() in {"true", "false"}:
        return "bool"

    # Detect multi-raster stack parameters (e.g., 'inputs' with list defaults)
    if n in ("inputs", "input_rasters", "raster_list", "rasters"):
        # Check if default value appears to be a list/array
        if isinstance(default_value, list) or (isinstance(default_value, str) and default_value.startswith("[")):
            return "raster_layers_in"
        # Also check if description hints at multi-raster
        if any(tok in d for tok in ("multiple rasters", "raster stack", "raster list", "input rasters")):
            return "raster_layers_in"

    vector_input_name_tokens = (
        "input",
        "vector",
        "source",
        "path",
        "target",
        "join",
        "origin",
        "origins",
        "destination",
        "destinations",
        "barrier",
        "barriers",
        "network",
        "route",
        "routes",
        "line",
        "lines",
        "point",
        "points",
        "polygon",
        "polygons",
    )

    vector_input_desc_tokens = (
        "layer",
        "vector",
        "features",
        "network",
        "point layer",
        "line layer",
        "polygon layer",
    )

    # Strong bool cue: descriptions that start with "if true" or "if false"
    # take priority even if the parameter name looks like an output path.
    if d.startswith(("if true", "if false", "if set", "if enabled", "if disabled",
                     "when true", "when false", "when set")):
        return "bool"

    if n in ("unit", "units", "mode", "method", "algorithm", "resample"):
        return "string"
    if "units:" in d or "output units" in d or "allowed values" in d:
        return "string"

    if _looks_like_output(n, d):
        if any(tok in n or tok in d for tok in ("raster", "dem", "grid", "geotiff", "tif")):
            return "raster_out"
        if any(tok in n or tok in d for tok in ("vector", "shp", "geojson", "topojson", "geopackage", "features")):
            return "vector_out"
        if any(tok in n or tok in d for tok in ("lidar", "las", "laz", "zlidar", "copc", "e57", "ply")):
            return "lidar_out"
        return "file_out"

    # Numeric hints must be evaluated before raster/vector/file hints, otherwise
    # phrases like "DEM units" can incorrectly force file-picker widgets.
    if n in {
        "filter_size",
        "kernel_size",
        "window_size",
        "outer_iterations",
        "iterations",
    }:
        return "int"

    if n in {
        "convergence_threshold",
        "normal_diff_threshold",
        "threshold",
        "lambda",
        "z_factor",
    }:
        return "double"

    if any(tok in n or tok in d for tok in ("integer", " int ", "count", "iterations", "num_", "_size", "size in cells")):
        return "int"

    # Restrict ambiguous tokens to name-only to prevent substring false-positives
    # in descriptive text (e.g. "calibration" contains "ratio", "factor" in prose).
    name_only_double = ("factor", "ratio", "angle", "weight", "radius", "percent")
    desc_safe_double = ("distance", "threshold")
    if any(tok in n for tok in name_only_double + desc_safe_double):
        return "double"
    if any(tok in d for tok in desc_safe_double + ("units",)):
        return "double"

    # Treat raster/vector/lidar as file/layer inputs only when the parameter
    # name itself looks like an input/path key.
    if (
        any(tok in n for tok in ("input", "raster", "dem", "grid", "source", "path"))
        and any(tok in n or tok in d for tok in ("raster", "dem", "grid", "geotiff", "tif"))
    ):
        return "raster_in"

    if (
        any(tok in n for tok in vector_input_name_tokens)
        and (
            any(tok in n or tok in d for tok in ("shp", "geojson", "topojson", "geopackage"))
            or any(
                tok in d
                for tok in (
                    "vector",
                    "features",
                    "layer",
                    "network layer",
                    "point layer",
                    "line layer",
                    "polygon layer",
                )
            )
        )
    ):
        return "vector_in"

    if (
        any(tok in n for tok in vector_input_name_tokens)
        and any(tok in d for tok in vector_input_desc_tokens)
    ):
        return "vector_in"

    if (
        any(tok in n for tok in ("input", "lidar", "source", "path"))
        and any(tok in n or tok in d for tok in ("lidar", "las", "laz", "zlidar"))
    ):
        return "file_in"

    if n.startswith(("is_", "has_", "use_", "enable_", "auto_")) or "true/false" in d:
        return "bool"

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


class _StreamFeedbackAdapter:
    """Translates runtime stream events into QGIS feedback actions.

    Handles progress, message, warning, error, and info event types with
    appropriate severity routing.  Also supports cancellation polling: if the
    provided *feedback* object reports cancellation the adapter sets an
    internal flag that the caller should check between stream chunks.
    """

    _SEVERITY_WARNING = frozenset({"warning", "warn"})
    _SEVERITY_ERROR = frozenset({"error", "fatal", "critical"})
    _SEVERITY_CONSOLE = frozenset({"console", "debug", "trace", "verbose"})

    def __init__(self, feedback, tool_name: str = ""):
        self._feedback = feedback
        self._tool_name = tool_name
        self.cancelled = False

    def __call__(self, event: Any) -> None:
        """Process one stream event payload (JSON string or dict)."""
        feedback = self._feedback
        if feedback is None:
            return

        # Honour cancellation between events.
        is_canceled = getattr(feedback, "isCanceled", None)
        if callable(is_canceled):
            try:
                if bool(is_canceled()):
                    self.cancelled = True
                    return
            except Exception:
                pass

        try:
            payload = json.loads(event) if isinstance(event, str) else event
        except Exception:
            # Non-JSON text lines are treated as plain info messages.
            if isinstance(event, str) and event.strip():
                push_info = getattr(feedback, "pushInfo", None)
                if callable(push_info):
                    push_info(str(event).strip())
            return
        if not isinstance(payload, dict):
            return

        event_type = str(payload.get("type", "message")).lower()

        if event_type == "progress":
            self._handle_progress(payload)
        elif event_type in self._SEVERITY_ERROR:
            self._handle_message(payload, severity="error")
        elif event_type in self._SEVERITY_WARNING:
            self._handle_message(payload, severity="warning")
        elif event_type in self._SEVERITY_CONSOLE:
            self._handle_message(payload, severity="console")
        else:
            # "message", "info", or any unrecognised type → info.
            self._handle_message(payload, severity="info")

    # ------------------------------------------------------------------
    def _handle_progress(self, payload: dict) -> None:
        feedback = self._feedback
        try:
            pct_raw = payload.get("percent") or payload.get("value") or 0.0
            pct = float(pct_raw)
            # Some runtimes emit fractional 0-1 values.
            if pct <= 1.0 and pct >= 0.0:
                pct *= 100.0
            pct = max(0.0, min(100.0, pct))
            set_progress = getattr(feedback, "setProgress", None)
            if callable(set_progress):
                set_progress(pct)
        except Exception:
            pass

        # Progress events may also carry a message.
        msg = payload.get("message") or payload.get("label")
        if msg:
            self._push(str(msg), severity="info")

    def _handle_message(self, payload: dict, severity: str) -> None:
        msg = payload.get("message") or payload.get("text") or payload.get("msg")
        if not msg:
            return
        msg_str = str(msg)
        self._push(msg_str, severity=severity)

        # Recover numeric progress from message text when present
        # (some tools only emit textual progress lines).
        if severity not in self._SEVERITY_ERROR:
            m = re.search(r"(-?\d+(?:\.\d+)?)\s*%", msg_str)
            if m:
                try:
                    pct = max(0.0, min(100.0, float(m.group(1))))
                    set_progress = getattr(self._feedback, "setProgress", None)
                    if callable(set_progress):
                        set_progress(pct)
                except Exception:
                    pass

    def _push(self, message: str, severity: str) -> None:
        feedback = self._feedback
        if severity in self._SEVERITY_ERROR:
            push = getattr(feedback, "reportError", None)
            if callable(push):
                try:
                    push(message)
                    return
                except Exception:
                    pass
            # Fallback when reportError is unavailable.
            push_warn = getattr(feedback, "pushWarning", None)
            if callable(push_warn):
                try:
                    push_warn(f"[ERROR] {message}")
                    return
                except Exception:
                    pass
        elif severity == "warning":
            push_warn = getattr(feedback, "pushWarning", None)
            if callable(push_warn):
                try:
                    push_warn(message)
                    return
                except Exception:
                    pass
        elif severity == "console":
            push_console = getattr(feedback, "pushConsoleInfo", None)
            if callable(push_console):
                try:
                    push_console(message)
                    return
                except Exception:
                    pass
        # Default / info path.
        push_info = getattr(feedback, "pushInfo", None)
        if callable(push_info):
            try:
                push_info(message)
            except Exception:
                pass


def _resolve_render_hint_for_output(
    hints: dict[str, str],
    output_key: str,
    output_value: Any,
) -> str:
    key = str(output_key).strip()
    value = output_value if isinstance(output_value, str) else ""
    hint = hints.get(key)
    if hint is None and key.endswith("_path"):
        # Common aliasing between runtime keys and plugin-facing output keys.
        base_key = key[:-5]
        hint = hints.get(base_key)
    if hint is None and key in {"path", "output"}:
        # Treat generic output aliases as interchangeable for render hints.
        hint = hints.get("output") or hints.get("path")
    if hint is None:
        hint = hints.get("raster")
    if hint is None and _is_raster_path(value):
        hint = hints.get("default_raster")
    return str(hint or "").strip().lower()


def _multiscale_topographic_position_palette_classes():
    class_type = getattr(QgsPalettedRasterRenderer, "Class", None)
    if class_type is None:
        return None

    classes = []
    for value, label, color_hex in _MULTISCALE_TOPOGRAPHIC_POSITION_CLASSES:
        try:
            classes.append(class_type(value, QColor(color_hex), label))
        except Exception:
            return None
    return classes


def _apply_categorical_raster_render_hint(path: str, feedback, tool_id: str = "", output_key: str = "") -> None:
    # Best effort only: QGIS APIs differ by host version and build.
    if not path or not _is_raster_path(path):
        return

    try:
        layer = QgsRasterLayer(path, "whitebox_render_hint_tmp")
    except Exception:
        return

    is_valid = getattr(layer, "isValid", None)
    if callable(is_valid):
        try:
            if not bool(is_valid()):
                return
        except Exception:
            return

    provider_getter = getattr(layer, "dataProvider", None)
    provider = provider_getter() if callable(provider_getter) else None
    if provider is None:
        return

    classes = None
    if str(tool_id or "").strip().lower() == "multiscale_topographic_position_class" and str(output_key or "").strip().lower() in {"path", "output", ""}:
        classes = _multiscale_topographic_position_palette_classes()

    if not classes:
        class_data_fn = getattr(QgsPalettedRasterRenderer, "classDataFromRaster", None)
        if not callable(class_data_fn):
            return

        for args in ((provider, 1, 256), (provider, 1), (layer, 1, 256), (layer, 1)):
            try:
                classes = class_data_fn(*args)
                if classes:
                    break
            except Exception:
                continue

    if not classes:
        return

    try:
        renderer = QgsPalettedRasterRenderer(provider, 1, classes)
        set_renderer = getattr(layer, "setRenderer", None)
        if callable(set_renderer):
            set_renderer(renderer)
        # Also apply the renderer to already-loaded project layers that match
        # this output path, which is what users actually see in the Layers panel.
        try:
            norm_target = os.path.normcase(os.path.normpath(str(path)))
            project_instance = getattr(QgsProject, "instance", None)
            project = project_instance() if callable(project_instance) else None
            map_layers = getattr(project, "mapLayers", None)
            layers_dict = map_layers() if callable(map_layers) else {}
            if isinstance(layers_dict, dict):
                for lyr in layers_dict.values():
                    try:
                        source_getter = getattr(lyr, "source", None)
                        lyr_source = source_getter() if callable(source_getter) else ""
                        # Strip provider query params (e.g. "|layername=...")
                        lyr_path = str(lyr_source).split("|", 1)[0]
                        if not lyr_path:
                            continue
                        norm_lyr = os.path.normcase(os.path.normpath(lyr_path))
                        if norm_lyr != norm_target:
                            continue
                        lyr_provider_getter = getattr(lyr, "dataProvider", None)
                        lyr_provider = lyr_provider_getter() if callable(lyr_provider_getter) else None
                        if lyr_provider is None:
                            continue
                        lyr_renderer = QgsPalettedRasterRenderer(lyr_provider, 1, classes)
                        lyr_set_renderer = getattr(lyr, "setRenderer", None)
                        if callable(lyr_set_renderer):
                            lyr_set_renderer(lyr_renderer)
                        trigger_repaint = getattr(lyr, "triggerRepaint", None)
                        if callable(trigger_repaint):
                            trigger_repaint()
                    except Exception:
                        continue
        except Exception:
            pass
        save_named_style = getattr(layer, "saveNamedStyle", None)
        if callable(save_named_style):
            qml_path = f"{path}.qml"
            save_named_style(qml_path)
            if feedback is not None:
                push_info = getattr(feedback, "pushInfo", None)
                if callable(push_info):
                    push_info(f"Applied categorical render hint and wrote style: {qml_path}")
    except Exception:
        return


def _record_recent_tool_execution(tool_id: str, max_recent: int = 8) -> None:
    key = "whitebox_workflows/recent_tools"
    cleaned: list[str] = []
    try:
        settings = QSettings()
        raw = settings.value(key, "")
        if isinstance(raw, str) and raw.strip():
            data = json.loads(raw)
            if isinstance(data, list):
                cleaned = [str(v).strip() for v in data if str(v).strip()]
    except Exception:
        cleaned = []

    tid = str(tool_id or "").strip()
    if not tid:
        return
    cleaned = [v for v in cleaned if v != tid]
    cleaned.insert(0, tid)
    cleaned = cleaned[:max_recent]

    try:
        settings = QSettings()
        settings.setValue(key, json.dumps(cleaned))
    except Exception:
        pass


def _remove_existing_output_artifacts(path: str) -> None:
    target = str(path or "").strip()
    if not target:
        return
    if target.startswith("memory:"):
        return

    # Shapefile outputs require sidecar cleanup to behave like overwrite.
    lower = target.lower()
    if lower.endswith(".shp"):
        stem, _ext = os.path.splitext(target)
        patterns = [
            f"{stem}.shp",
            f"{stem}.shx",
            f"{stem}.dbf",
            f"{stem}.prj",
            f"{stem}.cpg",
            f"{stem}.qpj",
            f"{stem}.sbn",
            f"{stem}.sbx",
            f"{stem}.fbn",
            f"{stem}.fbx",
            f"{stem}.ain",
            f"{stem}.aih",
            f"{stem}.atx",
            f"{stem}.ixs",
            f"{stem}.mxs",
            f"{stem}.xml",
        ]
        failed: list[str] = []
        for patt in patterns:
            for f in glob.glob(patt):
                try:
                    if os.path.isfile(f):
                        os.remove(f)
                except Exception as exc:
                    failed.append(f"{f} ({exc})")
        if failed:
            raise RuntimeError(
                "Cannot overwrite existing shapefile output because one or more sidecar files are locked or not writable: "
                + "; ".join(failed)
            )
        return

    try:
        if os.path.isfile(target):
            os.remove(target)
    except Exception as exc:
        raise RuntimeError(
            f"Cannot overwrite existing output file '{target}'. It may be locked by QGIS or not writable. Detail: {exc}"
        )


def _looks_like_vector_file_path(path: str) -> bool:
    value = str(path or "").strip().lower()
    if not value:
        return False
    vector_exts = (
        ".gpkg",
        ".shp",
        ".geojson",
        ".topojson",
        ".json",
        ".sqlite",
        ".gml",
        ".kml",
        ".csv",
        ".fgb",
        ".dxf",
    )
    return value.endswith(vector_exts)


def _check_for_malformed_gpkg(gpkg_path: str) -> dict[str, Any]:
    """
    Detect and diagnose malformed GeoPackage schema issues.
    
    Returns a dict with keys:
      - is_gpkg: whether the file is a GeoPackage
      - is_readable: whether the file can be opened by SQLite
      - has_schema_error: whether a duplicate column error was detected
      - error_details: string describing the error if one was found
    """
    result = {
        "is_gpkg": False,
        "is_readable": False,
        "has_schema_error": False,
        "error_details": "",
    }

    gpkg_path = str(gpkg_path or "").strip()
    if not gpkg_path or ".gpkg" not in gpkg_path.lower():
        return result

    result["is_gpkg"] = True

    try:
        conn = sqlite3.connect(gpkg_path, timeout=2.0)
        conn.row_factory = sqlite3.Row
        cursor = conn.cursor()
        
        # Query gpkg_contents to list all layers
        cursor.execute("SELECT table_name FROM gpkg_contents WHERE data_type IN ('features', 'tiles')")
        layers = [row[0] for row in cursor.fetchall()]
        
        result["is_readable"] = True
        
        # For each layer, check if schema has duplicate fid column
        for layer_name in layers:
            try:
                cursor.execute(f"PRAGMA table_info({layer_name})")
                columns = [row[1].lower() for row in cursor.fetchall()]
                
                # Count how many columns are named 'fid'
                fid_count = sum(1 for c in columns if c == "fid")
                if fid_count > 1:
                    result["has_schema_error"] = True
                    result["error_details"] = (
                        f"GeoPackage '{os.path.basename(gpkg_path)}' has malformed schema in layer "
                        f"'{layer_name}': duplicate 'fid' column detected. This likely means it was written "
                        f"with an older version of Whitebox. You should regenerate this output using the "
                        f"current version of the tool."
                    )
                    break
            except sqlite3.OperationalError:
                # Layer table might not exist or be readable; skip
                pass
        
        conn.close()
    except sqlite3.DatabaseError as e:
        result["error_details"] = f"Cannot read GeoPackage schema: {str(e)}"
    except Exception as e:
        result["error_details"] = f"Unexpected error checking GeoPackage: {str(e)}"

    return result


def _normalize_vector_input_source(source: Any) -> str:
    """Normalize QGIS vector layer URIs to backend-friendly paths when possible."""
    value = str(source or "").strip()
    if not value:
        return ""

    # File-based OGR layers often include provider options such as
    # "|layername=..." that the backend vector reader does not accept.
    if "|" in value:
        candidate = value.split("|", 1)[0].strip()
        if _looks_like_vector_file_path(candidate):
            value = candidate

    # Some providers expose OGR-style URI strings with dbname='...'.
    dbname_match = re.search(r"dbname=['\"]([^'\"]+)['\"]", value, flags=re.IGNORECASE)
    if dbname_match:
        candidate = dbname_match.group(1).strip()
        if _looks_like_vector_file_path(candidate):
            value = candidate

    return value


def _actionable_runtime_error_message(raw_message: str) -> str:
    message = str(raw_message or "").strip() or "Unknown runtime error."
    lower = message.lower()
    recommendations: list[str] = []

    def _add(rec: str) -> None:
        if rec not in recommendations:
            recommendations.append(rec)

    if "include_pro=true requested" in lower and "does not include pro support" in lower:
        _add("Rebuild/install whitebox_workflows with Pro support (for example: ./scripts/dev_python_install.sh --pro) or disable Include Pro in plugin settings.")

    if (
        "legacy whitebox_workflows runtime" in lower
        or "requires whitebox_workflows next gen" in lower
        or "unexpected keyword argument 'include_pro'" in lower
        or "unexpected keyword argument 'tier'" in lower
        or "has no attribute 'runtimesession'" in lower
    ):
        _add("Activate a whitebox_workflows Next Gen (v2.x) runtime and refresh the catalog.")

    if "no external python interpreter was found" in lower:
        _add("Set WBW_EXTERNAL_PYTHON to a valid interpreter that can import whitebox_workflows.")

    if (
        "no module named 'whitebox_workflows'" in lower
        or "package is not available in this python environment" in lower
    ):
        _add("Install/activate whitebox_workflows in the runtime interpreter used by QGIS, then refresh catalog.")

    if "permission denied" in lower or "read-only file system" in lower:
        _add("Choose writable output paths and verify directory permissions.")

    if "no such file or directory" in lower:
        _add("Verify all input paths exist and output directories have been created.")

    if "gpkg" in lower and ("schema" in lower or "malformed" in lower or "no such table" in lower):
        _add("Use explicit non-temporary outputs when possible and validate GeoPackage schema before rerunning.")

    _add("Open Whitebox panel -> Runtime Diagnostics for interpreter and tier details.")

    if not recommendations:
        return message

    return message + "\n\nRecommended actions:\n- " + "\n- ".join(recommendations)


def _extract_backend_error_message(response: dict[str, Any]) -> str:
    error_value = response.get("error")
    if isinstance(error_value, str) and error_value.strip():
        return error_value.strip()

    status = str(response.get("status", "")).strip().lower()
    if status in {"error", "failed", "failure"}:
        for key in ("message", "detail", "details", "reason"):
            value = response.get(key)
            if isinstance(value, str) and value.strip():
                return value.strip()
        return f"Runtime reported failure status: {status}."

    return ""


def _materialize_raster_input_source(
    source: Any, feedback=None, temp_paths: list[str] | None = None
) -> str:
    """Convert JP2-like sources to temporary GeoTIFF to avoid decoder artifacts."""
    value = str(source or "").strip()
    if not value:
        return ""

    # Remove provider suffixes where present (e.g. "path.jp2|...").
    if "|" in value:
        value = value.split("|", 1)[0].strip()

    lower = value.lower()
    if not lower.endswith((".jp2", ".j2k", ".j2c", ".jpc")):
        return value

    if gdal is None:
        if feedback is not None:
            feedback.pushWarning(
                "GDAL Python bindings are unavailable; using JP2 directly. "
                "Convert to GeoTIFF first if output appears corrupted."
            )
        return value

    ds = gdal.Open(value, gdal.GA_ReadOnly)
    if ds is None:
        return value

    output_type = None
    try:
        band = ds.GetRasterBand(1)
        nbits = band.GetMetadataItem("NBITS", "IMAGE_STRUCTURE") if band is not None else None
        if nbits is not None and str(nbits).strip() == "15":
            output_type = gdal.GDT_UInt16
    except Exception:
        output_type = None

    fd, tmp_path = tempfile.mkstemp(prefix="wbw_raster_", suffix=".tif")
    os.close(fd)

    kwargs: dict[str, Any] = {
        "format": "GTiff",
        "creationOptions": ["COMPRESS=DEFLATE", "TILED=YES"],
    }
    if output_type is not None:
        kwargs["outputType"] = output_type

    out_ds = gdal.Translate(tmp_path, ds, **kwargs)
    ds = None
    if out_ds is None:
        try:
            os.remove(tmp_path)
        except Exception:
            pass
        return value
    out_ds = None

    if temp_paths is not None:
        temp_paths.append(tmp_path)

    if feedback is not None:
        feedback.pushInfo(f"Materialized JP2 input to temporary GeoTIFF: {tmp_path}")
    return tmp_path


def _materialize_vector_input_source(layer, source: Any, feedback=None) -> str:
    """Return a backend-friendly vector path, exporting URI-backed layers when needed."""
    raw_source = str(source or "").strip()
    normalized = _normalize_vector_input_source(raw_source)
    if not normalized:
        return ""

    # Check for malformed GeoPackage files before processing.
    diagnostic = _check_for_malformed_gpkg(normalized)
    if diagnostic.get("has_schema_error") and feedback is not None:
        push_warn = getattr(feedback, "pushWarning", None)
        if callable(push_warn):
            push_warn(diagnostic.get("error_details", ""))

    # Fast path for plain filesystem sources: if normalization succeeded and
    # the result looks like a file path, return it directly without materialization.
    # This handles cases where QGIS provides URIs like "path.gpkg|layername=..." that
    # normalize to plain paths.
    if _looks_like_vector_file_path(normalized):
        # If the file exists and is readable, use it directly.
        if os.path.isfile(normalized):
            return normalized

    if layer is None:
        return normalized

    writer_cls = QgsVectorFileWriter
    writer_name = getattr(writer_cls, "__name__", "")
    if writer_name in {"", "_Dummy"}:
        return normalized

    layer_name = "input_layer"
    name_getter = getattr(layer, "name", None)
    if callable(name_getter):
        try:
            layer_name = str(name_getter() or layer_name).strip() or layer_name
        except Exception:
            pass

    safe_layer_name = re.sub(r"[^A-Za-z0-9_]+", "_", layer_name).strip("_") or "input_layer"
    tmp_root = os.path.join(tempfile.gettempdir(), "wbw_qgis_vector_inputs")
    os.makedirs(tmp_root, exist_ok=True)
    tmp_path = os.path.join(tmp_root, f"{safe_layer_name}_{os.getpid()}.gpkg")

    try:
        if os.path.exists(tmp_path):
            os.remove(tmp_path)
    except Exception:
        pass

    # QGIS 3.20+ / 4.x pathway.
    try:
        save_opts = writer_cls.SaveVectorOptions()
        save_opts.driverName = "GPKG"
        save_opts.layerName = safe_layer_name[:63]
        save_opts.fileEncoding = "UTF-8"
        save_opts.onlySelectedFeatures = False
        save_opts.actionOnExistingFile = getattr(
            writer_cls,
            "CreateOrOverwriteFile",
            getattr(writer_cls, "CreateOrOverwriteLayer", 0),
        )

        project_instance = getattr(QgsProject, "instance", None)
        project = project_instance() if callable(project_instance) else None
        transform_context_getter = getattr(project, "transformContext", None)
        transform_context = transform_context_getter() if callable(transform_context_getter) else None

        write_v3 = getattr(writer_cls, "writeAsVectorFormatV3", None)
        if callable(write_v3) and transform_context is not None:
            write_res = write_v3(layer, tmp_path, transform_context, save_opts)
            err_code = write_res[0] if isinstance(write_res, tuple) else write_res
            no_error = getattr(writer_cls, "NoError", 0)
            if int(err_code) == int(no_error) and os.path.isfile(tmp_path):
                return tmp_path
    except Exception:
        pass

    # Legacy fallback for API variants.
    try:
        write_legacy = getattr(writer_cls, "writeAsVectorFormat", None)
        if callable(write_legacy):
            crs_getter = getattr(layer, "crs", None)
            crs = crs_getter() if callable(crs_getter) else None
            write_res = write_legacy(layer, tmp_path, "UTF-8", crs, "GPKG")
            err_code = write_res[0] if isinstance(write_res, tuple) else write_res
            no_error = getattr(writer_cls, "NoError", 0)
            if int(err_code) == int(no_error) and os.path.isfile(tmp_path):
                return tmp_path
    except Exception:
        pass

    if feedback is not None:
        push_warn = getattr(feedback, "pushWarning", None)
        if callable(push_warn):
            push_warn(
                "Could not materialize provider-backed vector source; "
                "continuing with normalized path fallback."
            )

    return normalized


class WhiteboxCatalogAlgorithm(QgsProcessingAlgorithm):
    def __init__(self, provider, manifest: dict[str, Any]):
        super().__init__()
        self._provider = provider
        self._manifest = manifest
        self._param_specs: list[dict[str, Any]] = []
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
        return _derive_group_name(self._manifest)

    def groupId(self):
        return _normalize_group_id(self.group())

    def icon(self):
        if self._provider is None:
            return None
        provider_icon = getattr(self._provider, "icon", None)
        if callable(provider_icon):
            try:
                return provider_icon()
            except Exception:
                return None
        return None

    def shortHelpString(self):
        summary = str(self._manifest.get("summary", "") or "")
        tool_id = self.name()
        help_provider = get_help_provider()
        help_excerpt = help_provider.get_tool_help_excerpt(tool_id)
        hint_text = _render_hint_summary(self._render_hints)
        if bool(self._manifest.get("locked", False)):
            reason = self._manifest.get("locked_reason", "license_tier_insufficient")
            parts = [summary] if summary else []
            if help_excerpt:
                parts.append(help_excerpt)
            parts.append(
                "This tool is visible in the catalog but locked for the current runtime tier.\n"
                f"Reason: {reason}."
            )
            if hint_text:
                parts.append(hint_text)
            return "\n\n".join(p for p in parts if p)

        parts = [summary] if summary else []
        if help_excerpt and help_excerpt != summary:
            parts.append(help_excerpt)
        if hint_text:
            parts.append(hint_text)
        return "\n\n".join(p for p in parts if p)

    def helpUrl(self):
        path = self._provider.help_path_for_tool(self.name()) if self._provider else ""
        if path:
            from pathlib import Path
            return Path(path).as_uri()
        # Fallback: get_help_url already returns a file:// URI
        return get_help_url(self.name())

    def initAlgorithm(self, _config=None):
        self._param_specs = []
        for p in self._manifest.get("params", []):
            name = p.get("name", "")
            if not name:
                continue
            description = p.get("description", name)
            required = bool(p.get("required", False))
            default_value = self._manifest.get("defaults", {}).get(name)
            if default_value is None and "default" in p:
                default_value = p.get("default")
            kind = _infer_kind(name, description, default_value)
            enum_options = _extract_enum_options(name, description, default_value)

            # If an output destination is ambiguous, bias to the tool family so
            # QGIS can treat it as a loadable layer destination.
            if kind == "file_out":
                cat = str(self._manifest.get("category", "")).lower()
                if "vector" in cat:
                    kind = "vector_out"
                elif "lidar" in cat:
                    kind = "lidar_out"
                elif any(tok in cat for tok in ("raster", "terrain", "hydrology")):
                    kind = "raster_out"

            if kind == "string" and len(enum_options) >= 2:
                kind = "enum"

            # Safety net: when a non-output parameter default looks like a file
            # path, prefer an empty file selector instead of a prefilled string.
            if kind == "string" and not _looks_like_output(name, description):
                if _looks_like_filepath_default(default_value):
                    kind = "file_in"
                    default_value = None

            # A parameter whose name pattern looks like an output but whose
            # description resolves to a clear option list is an enum, not a
            # file destination (e.g. output_id_mode: rgb | user_data | ...).
            if kind in ("file_out", "raster_out", "vector_out") and len(enum_options) >= 2:
                kind = "enum"

            # A param inferred as numeric can still be an enum when the description
            # provides an explicit option list (e.g. mode params whose names contain
            # words like "factor" or "ratio", or descriptions like "calibration"
            # that contain those as substrings).
            if kind in ("double", "int") and len(enum_options) >= 2:
                kind = "enum"

            output_description = str(description)
            if kind in ("file_out", "raster_out", "vector_out"):
                lower_desc = output_description.lower()
                is_mstp = self.name() == "multiscale_topographic_position_class"
                is_auxiliary_output = (
                    "report" in name.lower()
                    or "diagnostic" in name.lower()
                    or "report" in lower_desc
                    or "diagnostic" in lower_desc
                )
                if "optional" in lower_desc and not is_mstp and not is_auxiliary_output:
                    output_description = "Output destination path (required in QGIS plugin)."
                elif "required in qgis plugin" not in lower_desc:
                    output_description = f"{output_description} (required in QGIS plugin)."

            if kind == "raster_in":
                qgs_param = QgsProcessingParameterRasterLayer(
                    name,
                    description,
                    defaultValue=None,
                    optional=not required,
                )
            elif kind == "raster_layers_in":
                if QgsProcessingParameterMultipleRasterLayers is None:
                    # Last-resort fallback for API variants without a dedicated
                    # multi-raster parameter type.
                    qgs_param = QgsProcessingParameterString(
                        name,
                        description,
                        defaultValue="",
                        optional=not required,
                    )
                elif getattr(
                    QgsProcessingParameterMultipleRasterLayers,
                    "__name__",
                    "",
                ) == "QgsProcessingParameterMultipleLayers":
                    layer_type = getattr(QgsProcessing, "TypeRaster", -1)
                    qgs_param = QgsProcessingParameterMultipleRasterLayers(
                        name,
                        description,
                        layer_type,
                        defaultValue=None,
                        optional=not required,
                    )
                else:
                    qgs_param = QgsProcessingParameterMultipleRasterLayers(
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
                    defaultValue=_coerce_bool_default(default_value, False),
                    optional=not required,
                )
            elif kind == "int":
                numeric_default = (
                    _coerce_int_default(default_value, 0)
                    if default_value is not None
                    else None
                )
                qgs_param = QgsProcessingParameterNumber(
                    name,
                    description,
                    QgsProcessingParameterNumber.Integer,
                    defaultValue=numeric_default,
                    optional=not required,
                )
            elif kind == "double":
                numeric_default = (
                    _coerce_float_default(default_value, 0.0)
                    if default_value is not None
                    else None
                )
                qgs_param = QgsProcessingParameterNumber(
                    name,
                    description,
                    QgsProcessingParameterNumber.Double,
                    defaultValue=numeric_default,
                    optional=not required,
                )
            elif kind == "file_out":
                qgs_param = QgsProcessingParameterFileDestination(
                    name,
                    output_description,
                    defaultValue=None,
                    optional=False,
                )
            elif kind == "raster_out":
                qgs_param = QgsProcessingParameterRasterDestination(
                    name,
                    output_description,
                    defaultValue=None,
                    optional=False,
                )
            elif kind == "vector_out":
                qgs_param = QgsProcessingParameterVectorDestination(
                    name,
                    output_description,
                    defaultValue=None,
                    optional=False,
                )
            elif kind == "lidar_out":
                qgs_param = QgsProcessingParameterFileDestination(
                    name,
                    output_description,
                    fileFilter="LiDAR files (*.las *.laz *.zlidar *.copc *.e57 *.ply)",
                    defaultValue=None,
                    optional=False,
                )
            elif kind == "file_in":
                qgs_param = QgsProcessingParameterFile(
                    name,
                    description,
                    behavior=QgsProcessingParameterFile.File,
                    defaultValue=None,
                    optional=not required,
                )
            elif kind == "enum":
                default_index = 0
                if default_value is not None:
                    default_text = str(default_value).strip().lower()
                    for idx, opt in enumerate(enum_options):
                        if opt.lower() == default_text:
                            default_index = idx
                            break
                qgs_param = QgsProcessingParameterEnum(
                    name,
                    description,
                    options=enum_options,
                    defaultValue=default_index,
                    optional=not required,
                )
            else:
                qgs_param = QgsProcessingParameterString(
                    name,
                    description,
                    defaultValue=_coerce_string_default(default_value, ""),
                    optional=not required,
                )

            # Apply enriched parameter help and labels
            # Priority: curated descriptions > legacy help > defaults
            descriptions_provider = get_descriptions_provider()
            help_provider = get_help_provider()
            tool_id = self._manifest.get("id", "")
            
            # Try curated label first
            curated_label = descriptions_provider.get_parameter_label(tool_id, name)
            if curated_label:
                # Replace the parameter with updated description
                # First, update the existing parameter's description
                # Note: We need to recreate the parameter with new description
                # because QGIS doesn't allow changing description after creation
                old_desc = description
                description = curated_label
                
                # Recreate parameter with updated description
                if kind == "raster_in":
                    qgs_param = QgsProcessingParameterRasterLayer(
                        name,
                        description,
                        defaultValue=None,
                        optional=not required,
                    )
                elif kind == "raster_layers_in":
                    if QgsProcessingParameterMultipleRasterLayers is None:
                        qgs_param = QgsProcessingParameterString(
                            name,
                            description,
                            defaultValue="",
                            optional=not required,
                        )
                    elif getattr(
                        QgsProcessingParameterMultipleRasterLayers,
                        "__name__",
                        "",
                    ) == "QgsProcessingParameterMultipleLayers":
                        layer_type = getattr(QgsProcessing, "TypeRaster", -1)
                        qgs_param = QgsProcessingParameterMultipleRasterLayers(
                            name,
                            description,
                            layer_type,
                            defaultValue=None,
                            optional=not required,
                        )
                    else:
                        qgs_param = QgsProcessingParameterMultipleRasterLayers(
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
                        defaultValue=_coerce_bool_default(default_value, False),
                        optional=not required,
                    )
                elif kind == "int":
                    numeric_default = (
                        _coerce_int_default(default_value, 0)
                        if default_value is not None
                        else None
                    )
                    qgs_param = QgsProcessingParameterNumber(
                        name,
                        description,
                        QgsProcessingParameterNumber.Integer,
                        defaultValue=numeric_default,
                        optional=not required,
                    )
                elif kind == "double":
                    numeric_default = (
                        _coerce_float_default(default_value, 0.0)
                        if default_value is not None
                        else None
                    )
                    qgs_param = QgsProcessingParameterNumber(
                        name,
                        description,
                        QgsProcessingParameterNumber.Double,
                        defaultValue=numeric_default,
                        optional=not required,
                    )
                elif kind == "file_out":
                    qgs_param = QgsProcessingParameterFileDestination(
                        name,
                        description,
                        defaultValue=None,
                        optional=False,
                    )
                elif kind == "raster_out":
                    qgs_param = QgsProcessingParameterRasterDestination(
                        name,
                        description,
                        defaultValue=None,
                        optional=False,
                    )
                elif kind == "vector_out":
                    qgs_param = QgsProcessingParameterVectorDestination(
                        name,
                        description,
                        defaultValue=None,
                        optional=False,
                    )
                elif kind == "lidar_out":
                    qgs_param = QgsProcessingParameterFileDestination(
                        name,
                        description,
                        fileFilter="LiDAR files (*.las *.laz *.zlidar *.copc *.e57 *.ply)",
                        defaultValue=None,
                        optional=False,
                    )
                elif kind == "file_in":
                    qgs_param = QgsProcessingParameterFile(
                        name,
                        description,
                        behavior=QgsProcessingParameterFile.File,
                        defaultValue=None,
                        optional=not required,
                    )
                elif kind == "enum":
                    default_index = 0
                    if default_value is not None:
                        default_text = str(default_value).strip().lower()
                        for idx, opt in enumerate(enum_options):
                            if opt.lower() == default_text:
                                default_index = idx
                                break
                    qgs_param = QgsProcessingParameterEnum(
                        name,
                        description,
                        options=enum_options,
                        defaultValue=default_index,
                        optional=not required,
                    )
                else:
                    qgs_param = QgsProcessingParameterString(
                        name,
                        description,
                        defaultValue=_coerce_string_default(default_value, ""),
                        optional=not required,
                    )
            
            # Apply tooltip if available (from either source)
            curated_tooltip = descriptions_provider.get_parameter_tooltip(tool_id, name)
            if curated_tooltip:
                qgs_param.setHelp(curated_tooltip)
            elif help_provider.has_help(tool_id):
                param_help = help_provider.get_parameter_help(tool_id, name)
                if param_help:
                    qgs_param.setHelp(param_help)

            self.addParameter(qgs_param)
            self._param_specs.append(
                {
                    "name": name,
                    "kind": kind,
                    "required": required or kind in ("file_out", "raster_out", "vector_out", "lidar_out"),
                    "enum_options": enum_options,
                }
            )

    def _resolve_raster_layer_sources(self, parameters, name: str, context) -> list[str]:
        """Resolve multi-raster inputs across QGIS API variants."""

        def _normalize_paths(values: Any) -> list[str]:
            if values is None:
                return []
            if isinstance(values, str):
                tokens = [p.strip() for p in re.split(r"[;\n]", values) if p.strip()]
                return [t.strip("\"'") for t in tokens if t.strip("\"' ")]
            if isinstance(values, (list, tuple)):
                out: list[str] = []
                for item in values:
                    out.extend(_normalize_paths(item))
                return out
            return []

        def _layer_sources(layers: Any) -> list[str]:
            if not isinstance(layers, (list, tuple)):
                return []
            out: list[str] = []
            for lyr in layers:
                if lyr is None:
                    continue
                src_getter = getattr(lyr, "source", None)
                src = src_getter() if callable(src_getter) else ""
                if isinstance(src, str) and src.strip():
                    out.append(src)
            return out

        raster_list_getter = getattr(self, "parameterAsRasterLayerList", None)
        if callable(raster_list_getter):
            try:
                resolved = _layer_sources(raster_list_getter(parameters, name, context))
                if resolved:
                    return resolved
            except Exception:
                pass

        layer_list_getter = getattr(self, "parameterAsLayerList", None)
        if callable(layer_list_getter):
            try:
                resolved = _layer_sources(layer_list_getter(parameters, name, context))
                if resolved:
                    return resolved
            except Exception:
                pass

        string_list_getter = getattr(self, "parameterAsStringList", None)
        if callable(string_list_getter):
            try:
                resolved = _normalize_paths(string_list_getter(parameters, name, context))
                if resolved:
                    return resolved
            except Exception:
                pass

        value = self.parameterAsString(parameters, name, context)
        if value:
            resolved = _normalize_paths(value)
            if resolved:
                return resolved

        return _normalize_paths(parameters.get(name))

    def processAlgorithm(self, parameters, context, feedback):
        if bool(self._manifest.get("locked", False)):
            reason = self._manifest.get("locked_reason", "license_tier_insufficient")
            raise QgsProcessingException(
                "This tool is locked for the active runtime tier. "
                f"Reason: {reason}."
            )

        args: dict[str, Any] = {}
        temp_raster_inputs: list[str] = []

        manifest_tier = str(self._manifest.get("license_tier", "") or "").strip().lower()
        manifest_tier_name = str(self._manifest.get("license_tier_name", "") or "").strip().lower()
        effective_manifest_tier = manifest_tier_name or manifest_tier

        # Strict execution-path contract:
        # - Pro-tier tools run through Pro runtime only.
        # - Open/unknown-tier tools run through open runtime only.
        # This ensures Pro users can execute both Pro and open tools while
        # preventing open tools from being routed through Pro-only paths.
        pro_tiers = {"pro", "enterprise", "professional"}
        is_pro_tool = effective_manifest_tier in pro_tiers

        if is_pro_tool:
            if not bool(self._provider.include_pro):
                raise QgsProcessingException(
                    "This tool requires the Pro runtime path, but the provider is currently in open mode. "
                    "Enable Pro runtime (include_pro=true) and retry."
                )
            exec_include_pro = True
            exec_tier = "pro"
        else:
            exec_include_pro = False
            exec_tier = "open"

        projection_wrapper_ids = {
            "reproject_raster",
            "reproject_lidar",
            "assign_projection_raster",
            "assign_projection_vector",
            "assign_projection_lidar",
        }

        for spec in self._param_specs:
            name = str(spec.get("name", ""))
            kind = str(spec.get("kind", "string"))
            required = bool(spec.get("required", False))
            is_set = _is_param_explicitly_set(parameters, name)
            if kind == "raster_in":
                lyr = self.parameterAsRasterLayer(parameters, name, context)
                if lyr is not None:
                    args[name] = _materialize_raster_input_source(lyr.source(), feedback, temp_raster_inputs)
                else:
                    # Some QGIS builds provide a file path but no layer object.
                    value = self.parameterAsString(parameters, name, context)
                    if value:
                        args[name] = _materialize_raster_input_source(str(value), feedback, temp_raster_inputs)
                    elif required:
                        raise QgsProcessingException(f"Missing required raster input: {name}")
                    else:
                        continue
            elif kind == "raster_layers_in":
                paths = self._resolve_raster_layer_sources(parameters, name, context)
                if paths:
                    args[name] = [
                        _materialize_raster_input_source(path, feedback, temp_raster_inputs) for path in paths
                    ]
                elif required:
                    raise QgsProcessingException(f"Missing required raster inputs: {name}")
                else:
                    continue
            elif kind == "vector_in":
                lyr = self.parameterAsVectorLayer(parameters, name, context)
                if lyr is not None:
                    resolved_path = _materialize_vector_input_source(lyr, lyr.source(), feedback)
                    args[name] = resolved_path
                    # Also check for malformed GeoPackage on file-based inputs
                    diagnostic = _check_for_malformed_gpkg(resolved_path)
                    if diagnostic.get("has_schema_error"):
                        feedback.pushWarning(diagnostic.get("error_details", ""))
                else:
                    # Some QGIS builds provide a file path but no layer object.
                    value = self.parameterAsString(parameters, name, context)
                    if value:
                        normalized = _normalize_vector_input_source(value)
                        args[name] = normalized
                        # Check for malformed GeoPackage on file path inputs
                        diagnostic = _check_for_malformed_gpkg(normalized)
                        if diagnostic.get("has_schema_error"):
                            feedback.pushWarning(diagnostic.get("error_details", ""))
                    elif required:
                        raise QgsProcessingException(f"Missing required vector input: {name}")
                    else:
                        continue
            elif kind == "bool":
                if not required and not is_set:
                    continue
                args[name] = bool(self.parameterAsBool(parameters, name, context))
            elif kind == "int":
                if not required and not is_set:
                    continue
                args[name] = int(self.parameterAsInt(parameters, name, context))
            elif kind == "double":
                if not required and not is_set:
                    continue
                value = float(self.parameterAsDouble(parameters, name, context))
                if not math.isfinite(value):
                    if required:
                        raise QgsProcessingException(f"Invalid numeric value for required parameter: {name}")
                    continue
                args[name] = value
            elif kind in ("file_out", "raster_out", "vector_out", "lidar_out"):
                # Destination parameters should resolve to concrete output paths.
                value = self.parameterAsOutputLayer(parameters, name, context)
                if not value:
                    value = self.parameterAsString(parameters, name, context)
                if not value:
                    if required:
                        raise QgsProcessingException(f"Missing required output destination: {name}")
                    continue
                args[name] = str(value)
            elif kind == "enum":
                if not required and not is_set:
                    continue
                options = [str(o) for o in spec.get("enum_options", [])]
                idx = int(self.parameterAsInt(parameters, name, context))
                if idx < 0 or idx >= len(options):
                    if required:
                        raise QgsProcessingException(f"Invalid option selected for parameter: {name}")
                    continue
                args[name] = options[idx]
            else:
                if not required and not is_set:
                    continue
                value = self.parameterAsString(parameters, name, context)
                if not value:
                    if required:
                        raise QgsProcessingException(f"Missing required parameter: {name}")
                    continue
                args[name] = value

        # Defensive type coercion for manifests that provide defaults but no
        # explicit kind metadata (common in runtime catalogs).
        defaults = self._manifest.get("defaults", {})
        if isinstance(defaults, dict):
            for key, default_value in defaults.items():
                if key in args:
                    args[key] = _coerce_arg_from_default_type(args.get(key), default_value)

        # Ensure payload is JSON-safe and omit optional null/non-finite numeric values
        # (QGIS optional numeric widgets often surface as NaN when left blank).
        required_params = {
            str(spec.get("name", ""))
            for spec in self._param_specs
            if bool(spec.get("required", False))
        }
        for key in list(args.keys()):
            value = args.get(key)
            if value is None:
                if key in required_params:
                    raise QgsProcessingException(f"Missing required parameter: {key}")
                del args[key]
                continue
            if isinstance(value, float) and not math.isfinite(value):
                if key in required_params:
                    raise QgsProcessingException(f"Invalid numeric value for required parameter: {key}")
                del args[key]

        # Normalize provider URIs (e.g., ".gpkg|layername=...") for any
        # vector-like string argument, including params inferred as generic
        # file/string rather than explicit vector_in.
        for key, value in list(args.items()):
            if not isinstance(value, str):
                continue
            raw = value.strip()
            if not raw:
                continue
            if "|" not in raw and "dbname=" not in raw.lower():
                continue

            normalized = _normalize_vector_input_source(raw)
            if normalized == raw:
                continue
            if _looks_like_vector_file_path(normalized):
                args[key] = normalized

        session = None
        if self.name() not in projection_wrapper_ids:
            session = create_runtime_session(
                include_pro=exec_include_pro,
                tier=exec_tier,
            )

            # Normalize payload against runtime schema to avoid frontend/runtime
            # drift (unknown keys, textual nulls for optional numerics, etc.).
            args = _sanitize_args_using_runtime_schema(
                self.name(),
                args,
                session,
                feedback,
            )

        # Emit concise execution diagnostics for QA and reproducibility.
        push_info = getattr(feedback, "pushInfo", None)
        if callable(push_info):
            try:
                push_info(
                    f"{self.name()}: exec_mode include_pro={exec_include_pro}, tier={exec_tier}, arg_keys={','.join(sorted(args.keys()))}"
                )
            except Exception:
                pass

        # Bridge equivalent `input`/`dem` names only for tools that actually
        # declare a DEM parameter.
        has_dem_param = any(
            str(spec.get("name", "")).strip().lower() == "dem"
            for spec in self._param_specs
        )
        if has_dem_param:
            if "input" not in args and "dem" in args:
                args["input"] = args["dem"]
            if "dem" not in args and "input" in args:
                args["dem"] = args["input"]

        # Fallback for tools that strictly require `input` while manifests expose
        # a different primary input key.
        if "input" not in args:
            preferred_input_keys = (
                "input_raster",
                "source",
                "raster",
                "dem",
                "i",
            )
            for k in preferred_input_keys:
                v = args.get(k)
                if isinstance(v, str) and v.strip():
                    args["input"] = v
                    break

        if "input" not in args:
            for spec in self._param_specs:
                k = str(spec.get("name", ""))
                kind = str(spec.get("kind", ""))
                if kind not in ("raster_in", "file_in"):
                    continue
                v = args.get(k)
                if isinstance(v, str) and v.strip() and _is_raster_path(v):
                    args["input"] = v
                    break

        # Honor overwrite expectation from QGIS runs by clearing existing
        # declared output files before executing the tool.
        for spec in self._param_specs:
            name = str(spec.get("name", ""))
            kind = str(spec.get("kind", ""))
            if kind not in ("file_out", "raster_out", "vector_out", "lidar_out"):
                continue
            value = args.get(name)
            if isinstance(value, str) and value.strip():
                _remove_existing_output_artifacts(value)

        stream_adapter = _StreamFeedbackAdapter(feedback, tool_name=self.name())

        try:
            if self.name() in projection_wrapper_ids:
                response = run_projection_wrapper(
                    self.name(),
                    args,
                    include_pro=exec_include_pro,
                    tier=exec_tier,
                )
                response_raw = json.dumps(response)
            else:
                response_raw = session.run_tool_json_stream(
                    self.name(),
                    json.dumps(args),
                    stream_adapter,
                )
        except Exception as exc:
            # Reliability retry: if runtime tools fail, re-sanitize against
            # schema and retry exactly once before surfacing the error.
            if (
                session is not None
                and self.name() not in projection_wrapper_ids
            ):
                try:
                    retry_args = _sanitize_args_using_runtime_schema(
                        self.name(),
                        args,
                        session,
                        feedback,
                    )
                    if retry_args != args:
                        response_raw = session.run_tool_json_stream(
                            self.name(),
                            json.dumps(retry_args),
                            stream_adapter,
                        )
                        args = retry_args
                        exc = None
                except Exception:
                    pass

            if exc is None:
                pass
            else:
                for tmp_path in temp_raster_inputs:
                    try:
                        os.remove(tmp_path)
                    except Exception:
                        pass
                if isinstance(exc, QgsProcessingException):
                    raise
                if isinstance(exc, RuntimeBootstrapError):
                    raise QgsProcessingException(_actionable_runtime_error_message(str(exc))) from exc
                raise QgsProcessingException(_actionable_runtime_error_message(str(exc))) from exc

        if response_raw is None:
            for tmp_path in temp_raster_inputs:
                try:
                    os.remove(tmp_path)
                except Exception:
                    pass
            raise QgsProcessingException("Runtime call did not return a result payload.")

        if stream_adapter.cancelled:
            for tmp_path in temp_raster_inputs:
                try:
                    os.remove(tmp_path)
                except Exception:
                    pass
            raise QgsProcessingException("Processing cancelled.")

        response = (
            json.loads(response_raw)
            if isinstance(response_raw, str)
            else response_raw
        )
        if not isinstance(response, dict):
            for tmp_path in temp_raster_inputs:
                try:
                    os.remove(tmp_path)
                except Exception:
                    pass
            return {}

        backend_error_message = _extract_backend_error_message(response)
        if backend_error_message:
            for tmp_path in temp_raster_inputs:
                try:
                    os.remove(tmp_path)
                except Exception:
                    pass
            raise QgsProcessingException(_actionable_runtime_error_message(backend_error_message))

        _record_recent_tool_execution(self.name())

        if feedback.isCanceled():
            for tmp_path in temp_raster_inputs:
                try:
                    os.remove(tmp_path)
                except Exception:
                    pass
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
        for spec in self._param_specs:
            name = str(spec.get("name", ""))
            kind = str(spec.get("kind", ""))
            if kind in ("file_out", "raster_out", "vector_out", "lidar_out") and name in args:
                result[name] = args[name]

        # Assign-projection wrappers update files in place; emit an explicit
        # completion message so users get visible confirmation in the dialog log.
        assign_in_place_ids = {
            "assign_projection_raster",
            "assign_projection_vector",
            "assign_projection_lidar",
        }
        if self.name() in assign_in_place_ids:
            input_path = args.get("input")
            epsg_value = args.get("epsg")
            if isinstance(input_path, str) and input_path:
                feedback.pushInfo(
                    f"Projection metadata assigned in place: {input_path} (EPSG: {epsg_value})."
                )
                result.setdefault("updated_input", input_path)
            try:
                feedback.setProgress(100.0)
            except Exception:
                pass

        # Emit best-effort render hint messages for downstream display handling.
        for key, value in list(result.items()):
            if not isinstance(value, str) or not value:
                continue
            hint = _resolve_render_hint_for_output(self._render_hints, key, value)
            if (
                not hint
                and self.name() == "multiscale_topographic_position_class"
                and key in {"path", "output", "output_path"}
            ):
                hint = "categorical_raster"
            if hint:
                feedback.pushInfo(f"Render hint for {key}: {hint}")
                if hint in ("categorical", "categorical_raster"):
                    _apply_categorical_raster_render_hint(value, feedback, self.name(), key)

        # QGIS may load the output layer after processAlgorithm returns and ignore
        # temporary-layer styling. Attach a post-load hook for MSTP class output.
        if self.name() == "multiscale_topographic_position_class":
            try:
                output_path = result.get("output")
                if isinstance(output_path, str) and output_path.strip() and _is_raster_path(output_path):
                    details_getter = getattr(context, "layerToLoadOnCompletionDetails", None)
                    if callable(details_getter):
                        details = details_getter(output_path)
                        set_post = getattr(details, "setPostProcessor", None)
                        if callable(set_post):
                            set_post(_MstpStylePostProcessor.create())
            except Exception:
                pass

        for tmp_path in temp_raster_inputs:
            try:
                os.remove(tmp_path)
            except Exception:
                pass

        return result


def build_algorithms(provider, catalog: list[dict[str, Any]]) -> list[WhiteboxCatalogAlgorithm]:
    available = [m for m in catalog if not bool(m.get("locked", False))]
    locked = [m for m in catalog if bool(m.get("locked", False))]

    def _sort_key(item: dict[str, Any]) -> tuple[str, str, str]:
        return (
            _derive_group_name(item),
            str(item.get("display_name", "")),
            str(item.get("id", "")),
        )

    manifests = sorted(available, key=_sort_key) + sorted(locked, key=_sort_key)
    return [WhiteboxCatalogAlgorithm(provider, m) for m in manifests]