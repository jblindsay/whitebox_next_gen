from __future__ import annotations

import json
import os
import queue
import re
import threading
import tempfile
from typing import Any

from .algorithm import _materialize_raster_input_source
from .bootstrap import RuntimeBootstrapError, create_runtime_session
from .host_api import run_dialog

try:
    from qgis.PyQt.QtCore import QCoreApplication, QTimer, Qt
    from qgis.PyQt.QtWidgets import (
        QAbstractItemView,
        QCheckBox,
        QDialog,
        QFileDialog,
        QHBoxLayout,
        QLabel,
        QLineEdit,
        QListWidget,
        QListWidgetItem,
        QMessageBox,
        QPushButton,
        QProgressBar,
        QTextEdit,
        QVBoxLayout,
    )
    from qgis.PyQt.QtGui import QGuiApplication
    from qgis.core import QgsProject
except Exception:  # pragma: no cover
    QCoreApplication = None  # type: ignore[assignment]
    QTimer = None  # type: ignore[assignment]
    Qt = None  # type: ignore[assignment]
    QgsProject = None  # type: ignore[assignment]
    QGuiApplication = None  # type: ignore[assignment]

    class _Dummy:
        def __init__(self, *_args, **_kwargs):
            pass

        def __getattr__(self, _name):
            def _noop(*_args, **_kwargs):
                return None

            return _noop

    QAbstractItemView = _Dummy
    QCheckBox = _Dummy
    QDialog = _Dummy
    QFileDialog = _Dummy
    QHBoxLayout = _Dummy
    QLabel = _Dummy
    QLineEdit = _Dummy
    QListWidget = _Dummy
    QListWidgetItem = _Dummy
    QMessageBox = _Dummy
    QPushButton = _Dummy
    QProgressBar = _Dummy
    QTextEdit = _Dummy
    QVBoxLayout = _Dummy


_EXPRESSION_SNIPPETS: list[tuple[str, str]] = [
    ("Average", '(("A" + "B") / 2.0)'),
    ("Difference", '("A" - "B")'),
    ("Ratio", '("A" / ("B" + 0.000001))'),
    ("Normalized Difference", '(("A" - "B") / ("A" + "B" + 0.000001))'),
]

_OPERATOR_TOKENS: list[tuple[str, str]] = [
    ("+", "Add: A + B"),
    ("-", "Subtract: A - B"),
    ("*", "Multiply: A * B"),
    ("/", "Divide: A / B"),
    ("%", "Modulo: A % B"),
    ("^", "Power: A ^ B"),
    ("( )", "Parentheses grouping"),
    ("==", "Equal comparison"),
    ("!=", "Not equal comparison"),
    (">", "Greater than"),
    (">=", "Greater than or equal"),
    ("<", "Less than"),
    ("<=", "Less than or equal"),
    ("&&", "Logical AND"),
    ("||", "Logical OR"),
    ("!", "Logical NOT"),
    ("abs()", "Absolute value"),
    ("sqrt()", "Square root"),
    ("ln()", "Natural log"),
    ("log10()", "Base-10 log"),
    ("exp()", "Exponential"),
    ("sin()", "Sine"),
    ("cos()", "Cosine"),
    ("tan()", "Tangent"),
    ("min()", "Minimum"),
    ("max()", "Maximum"),
    ("if(cond,a,b)", "Conditional expression"),
]


def _selectable_text_flags():
    if Qt is None:
        return None

    # QGIS may expose either Qt5-style enums directly on Qt or Qt6-style enums
    # under Qt.TextInteractionFlag.
    mouse_flag = getattr(Qt, "TextSelectableByMouse", None)
    keyboard_flag = getattr(Qt, "TextSelectableByKeyboard", None)
    if mouse_flag is not None and keyboard_flag is not None:
        return mouse_flag | keyboard_flag

    interaction_flag = getattr(Qt, "TextInteractionFlag", None)
    if interaction_flag is None:
        return None

    mouse_flag = getattr(interaction_flag, "TextSelectableByMouse", None)
    keyboard_flag = getattr(interaction_flag, "TextSelectableByKeyboard", None)
    if mouse_flag is None or keyboard_flag is None:
        return None
    return mouse_flag | keyboard_flag


class RasterCalculatorAssistantDialog(QDialog):
    def __init__(self, iface, include_pro: bool = True, tier: str = "open", parent=None):
        super().__init__(parent or (iface.mainWindow() if iface is not None else None))
        self.iface = iface
        self.include_pro = bool(include_pro)
        self.tier = str(tier or "open")
        self._launch_params: dict[str, Any] | None = None
        self._layer_by_name: dict[str, Any] = {}
        self._temp_raster_inputs: list[str] = []
        self._direct_run_queue: queue.Queue | None = None
        self._direct_run_thread: threading.Thread | None = None
        self._direct_run_finished = False
        self._direct_run_response_raw: Any = None
        self._direct_run_error_title = ""
        self._direct_run_error_details = ""
        self._direct_run_context: dict[str, Any] = {}
        self._saw_progress_event = False

        self.setWindowTitle("Raster Calculator Assistant")
        resize = getattr(self, "resize", None)
        if callable(resize):
            resize(980, 680)

        self._build_ui()
        self._init_timers()
        self._refresh_raster_layers()
        self._select_active_raster()

    def _init_timers(self) -> None:
        self._direct_run_timer = QTimer(self) if QTimer is not None else None
        if self._direct_run_timer is not None:
            self._direct_run_timer.setInterval(50)
            self._direct_run_timer.timeout.connect(self._poll_direct_run_queue)

    def launch_params(self) -> dict[str, Any] | None:
        return dict(self._launch_params or {}) if self._launch_params else None

    def _build_ui(self) -> None:
        root = QVBoxLayout(self)

        intro = QLabel(
            "Build a raster expression here, then open the standard processing dialog "
            "with prefilled inputs and expression."
        )
        if hasattr(intro, "setWordWrap"):
            intro.setWordWrap(True)
        root.addWidget(intro)

        top = QHBoxLayout()
        root.addLayout(top)

        self.refresh_button = QPushButton("Refresh Layers")
        top.addWidget(self.refresh_button)

        self.auto_reproject_checkbox = QCheckBox("Auto reproject inputs")
        self.auto_reproject_checkbox.setChecked(True)
        top.addWidget(self.auto_reproject_checkbox)
        top.addStretch(1)

        output_row = QHBoxLayout()
        root.addLayout(output_row)
        output_row.addWidget(QLabel("Direct-run output (optional):"))
        self.output_path_edit = QLineEdit()
        self.output_path_edit.setPlaceholderText(
            "If empty, a temporary GeoTIFF is created for direct execution"
        )
        output_row.addWidget(self.output_path_edit, 1)
        self.browse_output_button = QPushButton("Browse")
        output_row.addWidget(self.browse_output_button)

        layers_header = QHBoxLayout()
        root.addLayout(layers_header)
        layers_header.addWidget(QLabel("Fallback input rasters (used only when expression names do not match layers):"))

        layers_body = QHBoxLayout()
        root.addLayout(layers_body)

        self.layers_list = QListWidget()
        if hasattr(self.layers_list, "setSelectionMode") and hasattr(QAbstractItemView, "ExtendedSelection"):
            self.layers_list.setSelectionMode(QAbstractItemView.ExtendedSelection)
        layers_body.addWidget(self.layers_list, 2)

        actions_col = QVBoxLayout()
        layers_body.addLayout(actions_col)
        self.insert_layer_token_button = QPushButton("Insert \"Layer\" Token")
        actions_col.addWidget(self.insert_layer_token_button)
        actions_col.addStretch(1)

        operators_col = QVBoxLayout()
        layers_body.addLayout(operators_col)
        operators_col.addWidget(QLabel("Operators and functions"))
        self.operators_list = QListWidget()
        for token, meaning in _OPERATOR_TOKENS:
            item = QListWidgetItem(f"{token}  -  {meaning}")
            item.setData(32, token)
            self.operators_list.addItem(item)
        operators_col.addWidget(self.operators_list, 1)
        self.insert_operator_button = QPushButton("Insert Operator")
        operators_col.addWidget(self.insert_operator_button)

        snippet_row = QHBoxLayout()
        root.addLayout(snippet_row)
        snippet_row.addWidget(QLabel("Expression snippets:"))
        for label, expr in _EXPRESSION_SNIPPETS:
            btn = QPushButton(label)
            btn.clicked.connect(lambda _checked=False, e=expr: self._insert_text(e))
            snippet_row.addWidget(btn)
        snippet_row.addStretch(1)

        self.expression_edit = QTextEdit()
        self.expression_edit.setPlaceholderText(
            'Example: (("DEM" - "FlowAccum") / ("DEM" + "FlowAccum" + 0.000001))'
        )
        root.addWidget(self.expression_edit, 2)

        self.status_label = QLabel("Select at least one raster and enter an expression.")
        if hasattr(self.status_label, "setWordWrap"):
            self.status_label.setWordWrap(True)
        root.addWidget(self.status_label)

        self.progress_bar = QProgressBar()
        self.progress_bar.setRange(0, 100)
        self.progress_bar.setValue(0)
        self.progress_bar.setFormat("Progress: %p%")
        self.progress_bar.setTextVisible(True)
        root.addWidget(self.progress_bar)

        self.error_details_edit = QTextEdit()
        self.error_details_edit.setReadOnly(True)
        self.error_details_edit.setPlaceholderText("Detailed runtime errors will appear here.")
        selectable_flags = _selectable_text_flags()
        if selectable_flags is not None:
            self.error_details_edit.setTextInteractionFlags(selectable_flags)
        self.error_details_edit.hide()
        root.addWidget(self.error_details_edit, 1)

        error_actions = QHBoxLayout()
        root.addLayout(error_actions)
        self.copy_error_button = QPushButton("Copy Error Details")
        self.copy_error_button.hide()
        error_actions.addStretch(1)
        error_actions.addWidget(self.copy_error_button)

        buttons = QHBoxLayout()
        root.addLayout(buttons)
        self.run_direct_button = QPushButton("Run Now (Direct)")
        self.open_processing_button = QPushButton("Open Processing Dialog")
        self.cancel_button = QPushButton("Cancel")
        buttons.addWidget(self.run_direct_button)
        buttons.addWidget(self.open_processing_button)
        buttons.addWidget(self.cancel_button)
        buttons.addStretch(1)

        self.refresh_button.clicked.connect(self._refresh_raster_layers)
        self.browse_output_button.clicked.connect(self._browse_output_path)
        self.insert_layer_token_button.clicked.connect(self._insert_selected_layer_token)
        self.layers_list.itemDoubleClicked.connect(lambda _item: self._insert_selected_layer_token())
        self.insert_operator_button.clicked.connect(self._insert_selected_operator)
        self.operators_list.itemDoubleClicked.connect(lambda _item: self._insert_selected_operator())
        self.copy_error_button.clicked.connect(self._copy_error_details)
        self.run_direct_button.clicked.connect(self._run_direct)
        self.open_processing_button.clicked.connect(self._accept_and_open_processing)
        self.cancel_button.clicked.connect(self.reject)

    def _project_layers(self) -> list[Any]:
        if QgsProject is None:
            return []
        project_instance = getattr(QgsProject, "instance", None)
        project = project_instance() if callable(project_instance) else None
        layer_map_getter = getattr(project, "mapLayers", None)
        layers = layer_map_getter().values() if callable(layer_map_getter) else []

        out = []
        for layer in layers:
            source_getter = getattr(layer, "source", None)
            if not callable(source_getter):
                continue
            band_count_getter = getattr(layer, "bandCount", None)
            if not callable(band_count_getter):
                continue
            try:
                _ = int(band_count_getter())
                src = str(source_getter() or "").strip()
            except Exception:
                continue
            if not src and not hasattr(layer, "dataProvider"):
                continue
            out.append(layer)

        out.sort(key=lambda lyr: str(getattr(lyr, "name", lambda: "")()).lower())
        return out

    def _refresh_raster_layers(self) -> None:
        existing_order = self._current_layer_names_ordered()
        self.layers_list.clear()
        self._layer_by_name = {}

        layers = self._project_layers()
        name_seen: dict[str, int] = {}
        for layer in layers:
            name_getter = getattr(layer, "name", None)
            base_name = str(name_getter() if callable(name_getter) else "Raster").strip() or "Raster"
            count = name_seen.get(base_name, 0)
            name_seen[base_name] = count + 1
            display_name = base_name if count == 0 else f"{base_name} ({count + 1})"

            item = QListWidgetItem(display_name)
            item.setSelected(display_name in existing_order)
            self.layers_list.addItem(item)
            self._layer_by_name[display_name] = layer

        self._set_status(f"Loaded {len(layers)} raster layer(s).", is_error=False)

    def _select_active_raster(self) -> None:
        active_getter = getattr(self.iface, "activeLayer", None)
        active = active_getter() if callable(active_getter) else None
        if active is None:
            return
        for i in range(self.layers_list.count()):
            item = self.layers_list.item(i)
            layer = self._layer_by_name.get(str(item.text()))
            if layer == active:
                item.setSelected(True)
                self.layers_list.setCurrentItem(item)
                break

    def _current_layer_names_ordered(self) -> list[str]:
        names: list[str] = []
        for i in range(self.layers_list.count()):
            item = self.layers_list.item(i)
            if item is not None and item.isSelected():
                names.append(str(item.text()))
        return names

    def _selected_layers_ordered(self) -> list[Any]:
        out: list[Any] = []
        for name in self._current_layer_names_ordered():
            layer = self._layer_by_name.get(name)
            if layer is not None:
                out.append(layer)
        return out

    def _insert_text(self, text: str) -> None:
        value = str(text or "")
        if not value:
            return
        cursor = self.expression_edit.textCursor()
        cursor.insertText(value)
        self.expression_edit.setFocus()

    def _insert_selected_layer_token(self) -> None:
        item = self.layers_list.currentItem()
        if item is None:
            return
        token = f'"{str(item.text())}"'
        self._insert_text(token)

    def _insert_selected_operator(self) -> None:
        item = self.operators_list.currentItem()
        if item is None:
            return
        token = str(item.data(32) or "").strip()
        if not token:
            return

        # Insert common call-style operators with cursor inside parentheses.
        if token.endswith("()"):
            self._insert_text(token)
            cursor = self.expression_edit.textCursor()
            cursor.movePosition(cursor.Left)
            self.expression_edit.setTextCursor(cursor)
            return

        if token == "( )":
            self._insert_text("()")
            cursor = self.expression_edit.textCursor()
            cursor.movePosition(cursor.Left)
            self.expression_edit.setTextCursor(cursor)
            return

        if token == "if(cond,a,b)":
            self._insert_text("if(, , )")
            cursor = self.expression_edit.textCursor()
            for _ in range(5):
                cursor.movePosition(cursor.Left)
            self.expression_edit.setTextCursor(cursor)
            return

        self._insert_text(f" {token} ")

    def _validate_inputs(self) -> str:
        expression = self._normalized_expression(self.expression_edit.toPlainText())
        if not expression:
            return "Enter an expression for raster_calculator."

        parsed_vars, parse_error = self._parse_quoted_variables(expression)
        if parse_error:
            return parse_error
        if not parsed_vars:
            return (
                "Expression must contain at least one quoted raster variable, "
                "e.g. '" + '"A" + "B"' + "'."
            )

        _layers, binding_error = self._bound_layers_for_expression(parsed_vars)
        if binding_error:
            return binding_error

        return ""

    def _parse_quoted_variables(self, expression: str) -> tuple[list[str], str]:
        text = self._normalized_expression(expression)
        if not text.strip():
            return [], ""

        # Match either single-quoted or double-quoted variable tokens.
        pattern = re.compile(r'"([^"\\]+)"|\'([^\'\\]+)\'')
        ordered: list[str] = []
        seen = set()
        for m in pattern.finditer(text):
            token = m.group(1) if m.group(1) is not None else m.group(2)
            token = str(token or "").strip()
            if not token:
                continue
            if token in seen:
                continue
            seen.add(token)
            ordered.append(token)

        # Detect unbalanced quotes (common source of backend parse failures).
        if text.count('"') % 2 != 0 or text.count("'") % 2 != 0:
            return [], "Expression contains unmatched quotation marks."

        return ordered, ""

    def _normalized_expression(self, expression: str) -> str:
        text = str(expression or "")
        # Normalize common smart-quote characters copied from docs/editors.
        text = (
            text.replace("\u201c", '"')
            .replace("\u201d", '"')
            .replace("\u2018", "'")
            .replace("\u2019", "'")
        )
        return text.strip()

    def _set_status(self, text: str, *, is_error: bool) -> None:
        self.status_label.setText(str(text))
        style = "color:#b03a2e;" if is_error else "color:#1e8449;"
        set_style = getattr(self.status_label, "setStyleSheet", None)
        if callable(set_style):
            set_style(style)
        if not is_error:
            self._clear_error_details()

    def _set_progress_percent(self, value: float) -> None:
        try:
            pct = max(0, min(100, int(round(float(value)))))
        except Exception:
            return
        if not self._saw_progress_event:
            self._saw_progress_event = True
            self.progress_bar.setRange(0, 100)
            self.progress_bar.setFormat("Progress: %p%")
        self.progress_bar.setValue(pct)

    def _parse_percent_from_message(self, text: str) -> float | None:
        match = re.search(r"(\d+(?:\.\d+)?)\s*%", str(text or ""))
        if not match:
            return None
        try:
            return float(match.group(1))
        except Exception:
            return None

    def _handle_stream_event(self, event: Any) -> None:
        payload: Any = event
        if isinstance(event, str):
            text = event.strip()
            if not text:
                return
            try:
                payload = json.loads(text)
            except Exception:
                pct = self._parse_percent_from_message(text)
                if pct is not None:
                    self._set_progress_percent(pct)
                self._set_status(text, is_error=False)
                if QCoreApplication is not None:
                    QCoreApplication.processEvents()
                return

        if not isinstance(payload, dict):
            return

        event_type = str(payload.get("type") or "").strip().lower()
        if event_type == "progress":
            percent = payload.get("percent")
            if isinstance(percent, (int, float)):
                if 0.0 <= float(percent) <= 1.0:
                    self._set_progress_percent(float(percent) * 100.0)
                else:
                    self._set_progress_percent(float(percent))
            message = payload.get("message")
            if isinstance(message, str) and message.strip():
                self._set_status(message.strip(), is_error=False)
        else:
            message = payload.get("message")
            if isinstance(message, str) and message.strip():
                pct = self._parse_percent_from_message(message)
                if pct is not None:
                    self._set_progress_percent(pct)
                self._set_status(message.strip(), is_error=False)

        if QCoreApplication is not None:
            QCoreApplication.processEvents()

    def _show_copiable_error(self, title: str, message: str) -> None:
        text = str(message or "").strip() or "Unknown error."
        self.status_label.setText(str(title or "Raster Calculator Error"))
        set_style = getattr(self.status_label, "setStyleSheet", None)
        if callable(set_style):
            set_style("color:#b03a2e;")
        self.error_details_edit.setPlainText(text)
        self.error_details_edit.show()
        self.copy_error_button.show()

    def _clear_error_details(self) -> None:
        self.error_details_edit.clear()
        self.error_details_edit.hide()
        self.copy_error_button.hide()

    def _copy_error_details(self) -> None:
        text = self.error_details_edit.toPlainText().strip()
        if not text or QGuiApplication is None:
            return
        clipboard = QGuiApplication.clipboard()
        if clipboard is None:
            return
        clipboard.setText(text)

    def _format_runtime_error_details(self, summary: str, args: dict[str, Any] | None = None) -> str:
        parts = [str(summary or "Unknown runtime error.").strip() or "Unknown runtime error."]
        if args:
            try:
                parts.append("Arguments:\n" + json.dumps(args, indent=2, sort_keys=True))
            except Exception:
                parts.append(f"Arguments: {args}")
        return "\n\n".join(parts)

    def _browse_output_path(self) -> None:
        try:
            path, _selected_filter = QFileDialog.getSaveFileName(
                self,
                "Choose Raster Calculator Output",
                "",
                "GeoTIFF (*.tif);;All files (*.*)",
            )
        except Exception:
            path = ""
        if path:
            self.output_path_edit.setText(str(path))

    def _selected_input_sources(self) -> list[str]:
        return self._selected_input_sources_for_layers(self._selected_layers_ordered())

    def _selected_input_sources_for_layers(self, layers: list[Any]) -> list[str]:
        out: list[str] = []
        for layer in layers:
            source_getter = getattr(layer, "source", None)
            source = source_getter() if callable(source_getter) else ""
            path = _materialize_raster_input_source(source, feedback=None, temp_paths=self._temp_raster_inputs)
            if path:
                out.append(path)
        return out

    def _resolve_layer_by_expression_var(self, var_name: str):
        token = str(var_name or "").strip()
        if not token:
            return None

        # 1) Exact display-name match.
        if token in self._layer_by_name:
            return self._layer_by_name[token]

        # 2) Case-insensitive display-name match.
        token_lc = token.lower()
        ci_display_matches = [
            layer for display, layer in self._layer_by_name.items() if str(display).strip().lower() == token_lc
        ]
        if len(ci_display_matches) == 1:
            return ci_display_matches[0]

        # 3) Match against base layer names (ignoring duplicate suffixes like " (2)").
        base_matches = []
        for display, layer in self._layer_by_name.items():
            base = str(display)
            if base.endswith(")") and " (" in base:
                head, tail = base.rsplit(" (", 1)
                if tail[:-1].isdigit():
                    base = head
            if base.strip().lower() == token_lc:
                base_matches.append(layer)
        if len(base_matches) == 1:
            return base_matches[0]

        return None

    def _bound_layers_for_expression(self, parsed_vars: list[str]) -> tuple[list[Any], str]:
        # Preferred path: bind variables directly by expression variable names.
        resolved: list[Any] = []
        unresolved: list[str] = []
        for var in parsed_vars:
            layer = self._resolve_layer_by_expression_var(var)
            if layer is None:
                unresolved.append(var)
            else:
                resolved.append(layer)

        if not unresolved:
            return resolved, ""

        # Fallback path: use manually selected layers in list order.
        selected = self._selected_layers_ordered()
        if len(selected) == len(parsed_vars):
            return selected, ""

        return [], (
            f"Could not map expression variable(s) {unresolved} to loaded raster layer names. "
            "Either use variable names that match layer names exactly, or select a fallback layer list "
            f"with exactly {len(parsed_vars)} raster(s)."
        )

    def _cleanup_temp_inputs(self) -> None:
        for path in self._temp_raster_inputs:
            try:
                os.remove(path)
            except Exception:
                pass
        self._temp_raster_inputs = []

    def _run_direct(self) -> None:
        if self._direct_run_thread is not None and self._direct_run_thread.is_alive():
            self._set_status("Raster Calculator is already running.", is_error=True)
            return

        problem = self._validate_inputs()
        if problem:
            self._set_status(problem, is_error=True)
            return

        output_path = self.output_path_edit.text().strip()
        temp_output_path = ""
        if not output_path:
            fd, temp_output_path = tempfile.mkstemp(prefix="wbw_raster_calc_", suffix=".tif")
            os.close(fd)
            output_path = temp_output_path

        expression = self._normalized_expression(self.expression_edit.toPlainText())
        parsed_vars, _parse_error = self._parse_quoted_variables(expression)
        bound_layers, binding_error = self._bound_layers_for_expression(parsed_vars)
        if binding_error:
            self._set_status(binding_error, is_error=True)
            return

        args = {
            "expression": expression,
            "inputs": self._selected_input_sources_for_layers(bound_layers),
            "auto_reproject": bool(self.auto_reproject_checkbox.isChecked()),
            "output": output_path,
        }

        self.progress_bar.setValue(0)
        self.progress_bar.setRange(0, 0)
        self.progress_bar.setFormat("Progress: running...")
        self._saw_progress_event = False
        self._set_status("Running raster_calculator...", is_error=False)
        self.run_direct_button.setEnabled(False)
        self.open_processing_button.setEnabled(False)

        self._direct_run_queue = queue.Queue()
        self._direct_run_finished = False
        self._direct_run_response_raw = None
        self._direct_run_error_title = ""
        self._direct_run_error_details = ""
        self._direct_run_context = {
            "args": args,
            "output_path": output_path,
        }

        def _worker() -> None:
            assert self._direct_run_queue is not None
            try:
                session = create_runtime_session(include_pro=self.include_pro, tier=self.tier)
                response_raw = session.run_tool_json_stream(
                    "raster_calculator",
                    json.dumps(args),
                    lambda evt: self._direct_run_queue.put(("event", evt)),
                )
                self._direct_run_queue.put(("response", response_raw))
            except RuntimeBootstrapError as exc:
                details = self._format_runtime_error_details(str(exc), args)
                self._direct_run_queue.put(("error", ("Raster Calculator Runtime Error", details)))
            except Exception as exc:
                details = self._format_runtime_error_details(f"Direct execution failed: {exc}", args)
                self._direct_run_queue.put(("error", ("Raster Calculator Runtime Error", details)))
            finally:
                self._direct_run_queue.put(("done", None))

        self._direct_run_thread = threading.Thread(target=_worker, daemon=True)
        self._direct_run_thread.start()
        if self._direct_run_timer is not None:
            self._direct_run_timer.start()

    def _poll_direct_run_queue(self) -> None:
        if self._direct_run_queue is None:
            return

        while True:
            try:
                kind, payload = self._direct_run_queue.get_nowait()
            except queue.Empty:
                break

            if kind == "event":
                self._handle_stream_event(payload)
            elif kind == "response":
                self._direct_run_response_raw = payload
            elif kind == "error":
                title, details = payload
                self._direct_run_error_title = str(title)
                self._direct_run_error_details = str(details)
            elif kind == "done":
                self._direct_run_finished = True

        thread_done = self._direct_run_thread is None or not self._direct_run_thread.is_alive()
        if self._direct_run_finished and thread_done:
            if self._direct_run_timer is not None:
                self._direct_run_timer.stop()
            self._finalize_direct_run()

    def _finalize_direct_run(self) -> None:
        self.run_direct_button.setEnabled(True)
        self.open_processing_button.setEnabled(True)
        if not self._saw_progress_event:
            self.progress_bar.setRange(0, 100)
            self.progress_bar.setValue(100)
            self.progress_bar.setFormat("Progress: completed")
        self._cleanup_temp_inputs()

        if self._direct_run_error_details:
            self._show_copiable_error(self._direct_run_error_title, self._direct_run_error_details)
            self._direct_run_thread = None
            self._direct_run_queue = None
            return

        response_raw = self._direct_run_response_raw
        output_path = str(self._direct_run_context.get("output_path") or "")
        args = self._direct_run_context.get("args")
        try:
            response = json.loads(response_raw) if isinstance(response_raw, str) else response_raw
        except Exception:
            response = None

        if not isinstance(response, dict):
            self._set_status("Direct execution returned an invalid response payload.", is_error=True)
            self._direct_run_thread = None
            self._direct_run_queue = None
            return

        status = str(response.get("status", "")).strip().lower()
        if status in {"error", "failed", "failure"}:
            message = str(response.get("error") or response.get("message") or "Execution failed.")
            self._show_copiable_error(
                "Raster Calculator Runtime Error",
                self._format_runtime_error_details(message, args if isinstance(args, dict) else None),
            )
            self._direct_run_thread = None
            self._direct_run_queue = None
            return

        final_output = output_path
        outputs = response.get("outputs") if isinstance(response.get("outputs"), dict) else response
        if isinstance(outputs, dict):
            out_val = outputs.get("output")
            if isinstance(out_val, dict):
                path = out_val.get("path")
                if isinstance(path, str) and path.strip():
                    final_output = path.strip()
            elif isinstance(out_val, str) and out_val.strip():
                final_output = out_val.strip()

        add_raster = getattr(self.iface, "addRasterLayer", None)
        if callable(add_raster) and final_output:
            name = os.path.splitext(os.path.basename(final_output))[0] or "raster_calculator_output"
            try:
                add_raster(final_output, name)
            except Exception:
                pass

        self._set_status(
            f"Direct execution completed. Output: {final_output}",
            is_error=False,
        )
        self._set_progress_percent(100.0)

        # Close assistant and prevent caller from opening processing dialog.
        self._launch_params = None
        self.accept()

        self._direct_run_thread = None
        self._direct_run_queue = None

    def _accept_and_open_processing(self) -> None:
        problem = self._validate_inputs()
        if problem:
            self._set_status(problem, is_error=True)
            return

        expression = self._normalized_expression(self.expression_edit.toPlainText())
        parsed_vars, _parse_error = self._parse_quoted_variables(expression)
        bound_layers, binding_error = self._bound_layers_for_expression(parsed_vars)
        if binding_error:
            self._set_status(binding_error, is_error=True)
            return

        self._launch_params = {
            "expression": expression,
            "inputs": bound_layers,
            "auto_reproject": bool(self.auto_reproject_checkbox.isChecked()),
        }
        self.accept()


def run_raster_calculator_assistant(iface, include_pro: bool = True, tier: str = "open") -> dict[str, Any] | None:
    dialog = RasterCalculatorAssistantDialog(
        iface,
        include_pro=include_pro,
        tier=tier,
        parent=iface.mainWindow() if iface is not None else None,
    )
    result = run_dialog(dialog)
    if result is None:
        return None
    accepted = bool(result)
    if not accepted:
        return {}
    return dialog.launch_params() or {}