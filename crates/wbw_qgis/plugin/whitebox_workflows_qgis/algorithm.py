from __future__ import annotations

import glob
import json
import os
import re
from typing import Any

from .bootstrap import create_runtime_session, run_projection_wrapper
from .help import get_help_url
from .help_provider import get_help_provider
from .descriptions_provider import get_descriptions_provider

try:
    from qgis.PyQt.QtCore import QSettings
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
        QgsRasterLayer,
    )
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
    QgsProcessingParameterNumber = _Dummy
    QgsProcessingParameterRasterLayer = _Dummy
    QgsProcessingParameterRasterDestination = _Dummy
    QgsProcessingParameterString = _Dummy
    QgsProcessingParameterVectorLayer = _Dummy
    QgsProcessingParameterVectorDestination = _Dummy
    QgsRasterLayer = _Dummy
    QgsPalettedRasterRenderer = _Dummy


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


def _derive_group_name(manifest: dict[str, Any]) -> str:
    base = str(manifest.get("category", "General") or "General")
    tags = [str(t).strip().lower() for t in manifest.get("tags", []) if str(t).strip()]
    tag_set = set(tags)

    # Preserve explicit non-broad categories UNLESS they have specific subcategory tags.
    if base not in {"Raster", "Vector", "Lidar", "Other"}:
        # Still check tags for terrain/hydrology/remote-sensing subcategories even
        # if the base category is already "Terrain", "Hydrology", etc.
        if base.lower() not in ("terrain", "hydrology", "raster", "vector", "lidar"):
            return base

    terrain_tags = {
        "terrain", "geomorphometry", "curvature", "roughness", "local-relief", "dem"
    }
    hydrology_tags = {
        "hydrology", "flow", "flow-accumulation", "flow-direction", "d8", "dinf", "streams", "stream_network", "depression", "watershed", "flowpath"
    }
    remote_tags = {
        "remote_sensing", "image", "classification", "filter", "convolution", "multiscale"
    }

    def _has(*names: str) -> bool:
        return any(n in tag_set for n in names)

    if tag_set.intersection(terrain_tags):
        if _has("curvature", "roughness", "geomorphometry"):
            return "Terrain - Geomorphometry"
        return "Terrain"

    if tag_set.intersection(hydrology_tags):
        if _has("flow-accumulation", "flow-direction", "d8", "dinf"):
            return "Hydrology - Flow"
        if _has("stream_network", "streams", "flowpath"):
            return "Hydrology - Streams"
        if _has("depression"):
            return "Hydrology - Depressions"
        return "Hydrology"

    if tag_set.intersection(remote_tags):
        if _has("classification", "knn"):
            return "Remote Sensing - Classification"
        if _has("filter", "convolution", "smoothing"):
            return "Remote Sensing - Filters"
        if _has("multiscale", "integral-image"):
            return "Remote Sensing - Multi-scale"
        return "Remote Sensing"

    if base == "Raster":
        if _has("math"):
            return "Raster - Math"
        if _has("statistics"):
            return "Raster - Statistics"
        if _has("overlay", "distance", "interpolation"):
            return "Raster - Analysis"
        return "Raster"

    if base == "Vector":
        if _has("network", "routing"):
            return "Vector - Network"
        if _has("linear-referencing", "linear_referencing"):
            return "Vector - Linear Referencing"
        return "Vector"

    if base == "Lidar":
        if _has("qa", "diagnostics"):
            return "LiDAR - QA"
        if _has("forestry", "biomass"):
            return "LiDAR - Forestry"
        return "LiDAR"

    return base


def _infer_kind(name: str, description: str) -> str:
    n = name.lower()
    d = description.lower()

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
        if any(tok in n or tok in d for tok in ("vector", "shp", "geojson", "geopackage", "features")):
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
        any(tok in n for tok in ("input", "vector", "source", "path"))
        and any(tok in n or tok in d for tok in ("vector", "shp", "geojson", "geopackage", "features"))
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
    hint = hints.get(key) or hints.get("raster")
    if hint is None and _is_raster_path(value):
        hint = hints.get("default_raster")
    return str(hint or "").strip().lower()


def _apply_categorical_raster_render_hint(path: str, feedback) -> None:
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

    class_data_fn = getattr(QgsPalettedRasterRenderer, "classDataFromRaster", None)
    if not callable(class_data_fn):
        return

    classes = None
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
        for patt in patterns:
            for f in glob.glob(patt):
                try:
                    if os.path.isfile(f):
                        os.remove(f)
                except Exception:
                    pass
        return

    try:
        if os.path.isfile(target):
            os.remove(target)
    except Exception:
        pass


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
        detailed_help_url = get_help_url(tool_id)
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
            if detailed_help_url:
                parts.append(f"Detailed help: {detailed_help_url}")
            return "\n\n".join(p for p in parts if p)

        parts = [summary] if summary else []
        if help_excerpt and help_excerpt != summary:
            parts.append(help_excerpt)
        if hint_text:
            parts.append(hint_text)
        if detailed_help_url:
            parts.append(f"Detailed help: {detailed_help_url}")
        return "\n\n".join(p for p in parts if p)

    def helpUrl(self):
        path = self._provider.help_path_for_tool(self.name()) if self._provider else ""
        if path:
            from pathlib import Path
            return Path(path).as_uri()
        return get_help_url(self.name())

    def initAlgorithm(self, _config=None):
        self._param_specs = []
        for p in self._manifest.get("params", []):
            name = p.get("name", "")
            if not name:
                continue
            description = p.get("description", name)
            required = bool(p.get("required", False))
            kind = _infer_kind(name, description)
            default_value = self._manifest.get("defaults", {}).get(name)
            if default_value is None and "default" in p:
                default_value = p.get("default")
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
                if "optional" in lower_desc:
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
                qgs_param = QgsProcessingParameterNumber(
                    name,
                    description,
                    QgsProcessingParameterNumber.Integer,
                    defaultValue=_coerce_int_default(default_value, 0),
                    optional=not required,
                )
            elif kind == "double":
                qgs_param = QgsProcessingParameterNumber(
                    name,
                    description,
                    QgsProcessingParameterNumber.Double,
                    defaultValue=_coerce_float_default(default_value, 0.0),
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
                    qgs_param = QgsProcessingParameterNumber(
                        name,
                        description,
                        QgsProcessingParameterNumber.Integer,
                        defaultValue=_coerce_int_default(default_value, 0),
                        optional=not required,
                    )
                elif kind == "double":
                    qgs_param = QgsProcessingParameterNumber(
                        name,
                        description,
                        QgsProcessingParameterNumber.Double,
                        defaultValue=_coerce_float_default(default_value, 0.0),
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

    def processAlgorithm(self, parameters, context, feedback):
        if bool(self._manifest.get("locked", False)):
            reason = self._manifest.get("locked_reason", "license_tier_insufficient")
            raise QgsProcessingException(
                "This tool is locked for the active runtime tier. "
                f"Reason: {reason}."
            )

        args: dict[str, Any] = {}

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
            if kind == "raster_in":
                lyr = self.parameterAsRasterLayer(parameters, name, context)
                if lyr is not None:
                    args[name] = lyr.source()
                else:
                    # Some QGIS builds provide a file path but no layer object.
                    value = self.parameterAsString(parameters, name, context)
                    if value:
                        args[name] = str(value)
                    elif required:
                        raise QgsProcessingException(f"Missing required raster input: {name}")
                    else:
                        continue
            elif kind == "vector_in":
                lyr = self.parameterAsVectorLayer(parameters, name, context)
                if lyr is not None:
                    args[name] = lyr.source()
                else:
                    # Some QGIS builds provide a file path but no layer object.
                    value = self.parameterAsString(parameters, name, context)
                    if value:
                        args[name] = str(value)
                    elif required:
                        raise QgsProcessingException(f"Missing required vector input: {name}")
                    else:
                        continue
            elif kind == "bool":
                args[name] = bool(self.parameterAsBool(parameters, name, context))
            elif kind == "int":
                args[name] = int(self.parameterAsInt(parameters, name, context))
            elif kind == "double":
                args[name] = float(self.parameterAsDouble(parameters, name, context))
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
                options = [str(o) for o in spec.get("enum_options", [])]
                idx = int(self.parameterAsInt(parameters, name, context))
                if idx < 0 or idx >= len(options):
                    if required:
                        raise QgsProcessingException(f"Invalid option selected for parameter: {name}")
                    continue
                args[name] = options[idx]
            else:
                value = self.parameterAsString(parameters, name, context)
                if not value:
                    if required:
                        raise QgsProcessingException(f"Missing required parameter: {name}")
                    continue
                args[name] = value

        session = None
        if self.name() not in projection_wrapper_ids:
            session = create_runtime_session(
                include_pro=self._provider.include_pro,
                tier=self._provider.tier,
            )

        # Bridge common equivalent parameter names across tool families.
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

        if self.name() in projection_wrapper_ids:
            response = run_projection_wrapper(
                self.name(),
                args,
                include_pro=self._provider.include_pro,
                tier=self._provider.tier,
            )
            response_raw = json.dumps(response)
        else:
            response_raw = session.run_tool_json_stream(
                self.name(),
                json.dumps(args),
                stream_adapter,
            )

        if stream_adapter.cancelled:
            raise QgsProcessingException("Processing cancelled.")

        response = (
            json.loads(response_raw)
            if isinstance(response_raw, str)
            else response_raw
        )
        if not isinstance(response, dict):
            return {}

        _record_recent_tool_execution(self.name())

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
            if hint:
                feedback.pushInfo(f"Render hint for {key}: {hint}")
                if hint in ("categorical", "categorical_raster"):
                    _apply_categorical_raster_render_hint(value, feedback)

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