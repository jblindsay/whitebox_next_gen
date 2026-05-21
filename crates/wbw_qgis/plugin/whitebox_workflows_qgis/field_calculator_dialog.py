from __future__ import annotations

import json
from typing import Any

from .algorithm import _materialize_vector_input_source
from .bootstrap import RuntimeBootstrapError, create_runtime_session

try:
    from qgis.PyQt.QtCore import Qt
    from qgis.PyQt.QtWidgets import (
        QCheckBox,
        QComboBox,
        QDialog,
        QGridLayout,
        QHBoxLayout,
        QLabel,
        QLineEdit,
        QListWidget,
        QListWidgetItem,
        QMessageBox,
        QPushButton,
        QSpinBox,
        QTableWidget,
        QTableWidgetItem,
        QTextEdit,
        QVBoxLayout,
        QWidget,
    )
    from qgis.core import QgsProject
except Exception:  # pragma: no cover
    Qt = None  # type: ignore[assignment]
    QgsProject = None  # type: ignore[assignment]

    class _Dummy:
        def __init__(self, *_args, **_kwargs):
            pass

        def __getattr__(self, _name):
            def _noop(*_args, **_kwargs):
                return None

            return _noop

    QCheckBox = _Dummy
    QComboBox = _Dummy
    QDialog = _Dummy
    QGridLayout = _Dummy
    QHBoxLayout = _Dummy
    QLabel = _Dummy
    QLineEdit = _Dummy
    QListWidget = _Dummy
    QListWidgetItem = _Dummy
    QMessageBox = _Dummy
    QPushButton = _Dummy
    QSpinBox = _Dummy
    QTableWidget = _Dummy
    QTableWidgetItem = _Dummy
    QTextEdit = _Dummy
    QVBoxLayout = _Dummy
    QWidget = _Dummy


_GEOMETRY_TOKENS: list[tuple[str, str]] = [
    ("$area", "$area"),
    ("$perimeter", "$perimeter"),
    ("$length", "$length"),
    ("$centroid_x", "$centroid_x"),
    ("$centroid_y", "$centroid_y"),
    ("NULL", "NULL"),
]

_PRESET_EXPRESSIONS: list[tuple[str, str]] = [
    (
        "Road speed from TYPE",
        "UPDATE roads SET SPEED = CASE\n"
        "  WHEN TYPE == 'motorway' THEN 100\n"
        "  WHEN TYPE == 'primary' THEN 80\n"
        "  WHEN TYPE == 'collector' THEN 60\n"
        "  ELSE 40\n"
        "END",
    ),
    (
        "Conditional area class",
        "CASE\n"
        "  WHEN $area >= 100000 THEN 'regional'\n"
        "  WHEN $area >= 10000 THEN 'local'\n"
        "  ELSE 'site'\n"
        "END",
    ),
    (
        "Null-safe text fallback",
        "CASE WHEN NAME IS NULL THEN 'unnamed' ELSE NAME END",
    ),
    (
        "Cast numeric id to text",
        "CAST(PARCEL_ID AS text)",
    ),
]

_SNIPPET_LIBRARY: dict[str, list[tuple[str, str]]] = {
    "Transportation": [
        (
            "Speed from TYPE",
            "CASE\n"
            "  WHEN TYPE == 'motorway' THEN 100\n"
            "  WHEN TYPE == 'primary' THEN 80\n"
            "  WHEN TYPE == 'collector' THEN 60\n"
            "  ELSE 40\n"
            "END",
        ),
        (
            "Impedance from SPEED",
            "CASE WHEN SPEED IS NULL OR SPEED <= 0 THEN NULL ELSE $length / SPEED END",
        ),
    ],
    "Hydrology and Terrain": [
        (
            "Drainage class from area",
            "CASE\n"
            "  WHEN $area >= 500000 THEN 'major'\n"
            "  WHEN $area >= 100000 THEN 'intermediate'\n"
            "  ELSE 'minor'\n"
            "END",
        ),
        (
            "Slope bins",
            "CASE\n"
            "  WHEN SLOPE < 2 THEN 'flat'\n"
            "  WHEN SLOPE < 8 THEN 'gentle'\n"
            "  WHEN SLOPE < 20 THEN 'moderate'\n"
            "  ELSE 'steep'\n"
            "END",
        ),
    ],
    "Data quality": [
        (
            "Null-safe name",
            "CASE WHEN NAME IS NULL OR NAME == '' THEN 'unnamed' ELSE NAME END",
        ),
        (
            "Quality flag",
            "CASE WHEN SCORE IS NULL THEN 'missing' WHEN SCORE < 0 THEN 'invalid' ELSE 'ok' END",
        ),
    ],
    "Type conversion": [
        ("Cast to integer", "CAST(VALUE AS integer)"),
        ("Cast to float", "CAST(VALUE AS float)"),
        ("Cast to text", "CAST(VALUE AS text)"),
    ],
}


class FieldCalculatorAssistantDialog(QDialog):
    def __init__(self, iface, include_pro: bool = True, tier: str = "open", parent=None):
        super().__init__(parent or (iface.mainWindow() if iface is not None else None))
        self.iface = iface
        self.include_pro = bool(include_pro)
        self.tier = str(tier or "open")
        self._launch_params: dict[str, Any] | None = None
        self._layer_by_index: dict[int, Any] = {}

        self.setWindowTitle("Field Calculator Assistant")
        resize = getattr(self, "resize", None)
        if callable(resize):
            resize(1080, 760)

        self._build_ui()
        self._refresh_layers()
        self._select_active_layer()

    def launch_params(self) -> dict[str, Any] | None:
        return dict(self._launch_params or {}) if self._launch_params else None

    def _build_ui(self) -> None:
        root = QVBoxLayout(self)

        intro = QLabel(
            "Build and preview SQL-style field expressions here, then hand the "
            "configured parameters off to the standard Whitebox processing dialog for the final run."
        )
        if hasattr(intro, "setWordWrap"):
            intro.setWordWrap(True)
        root.addWidget(intro)

        top_grid = QGridLayout()
        root.addLayout(top_grid)

        top_grid.addWidget(QLabel("Input layer"), 0, 0)
        self.layer_combo = QComboBox()
        top_grid.addWidget(self.layer_combo, 0, 1)
        self.refresh_layers_button = QPushButton("Refresh")
        top_grid.addWidget(self.refresh_layers_button, 0, 2)

        top_grid.addWidget(QLabel("Target field"), 1, 0)
        self.field_name_edit = QLineEdit()
        self.field_name_edit.setPlaceholderText("e.g. SPEED")
        top_grid.addWidget(self.field_name_edit, 1, 1)

        top_grid.addWidget(QLabel("Output type"), 1, 2)
        self.field_type_combo = QComboBox()
        self.field_type_combo.addItems(["float", "integer", "text"])
        top_grid.addWidget(self.field_type_combo, 1, 3)

        top_grid.addWidget(QLabel("Preview rows"), 2, 0)
        self.preview_rows_spin = QSpinBox()
        self.preview_rows_spin.setRange(1, 50)
        self.preview_rows_spin.setValue(8)
        top_grid.addWidget(self.preview_rows_spin, 2, 1)

        self.overwrite_checkbox = QCheckBox("Overwrite existing field")
        self.overwrite_checkbox.setChecked(True)
        top_grid.addWidget(self.overwrite_checkbox, 2, 2, 1, 2)

        preset_row = QHBoxLayout()
        root.addLayout(preset_row)
        preset_row.addWidget(QLabel("Starter"))
        self.preset_combo = QComboBox()
        self.preset_combo.addItem("Select an example...")
        for label, _expr in _PRESET_EXPRESSIONS:
            self.preset_combo.addItem(label)
        preset_row.addWidget(self.preset_combo, 1)
        self.load_preset_button = QPushButton("Load")
        preset_row.addWidget(self.load_preset_button)

        self.expression_edit = QTextEdit()
        self.expression_edit.setPlaceholderText(
            "Examples:\n"
            "CASE WHEN TYPE == 'motorway' THEN 100 ELSE 60 END\n\n"
            "UPDATE roads SET SPEED = CASE WHEN TYPE == 'primary' THEN 80 ELSE 40 END WHERE ACTIVE == 1"
        )
        root.addWidget(self.expression_edit, 2)

        tokens_row = QHBoxLayout()
        root.addLayout(tokens_row, 1)

        fields_box = QWidget()
        fields_layout = QVBoxLayout(fields_box)
        fields_layout.addWidget(QLabel("Fields"))
        self.fields_list = QListWidget()
        fields_layout.addWidget(self.fields_list)
        self.insert_field_button = QPushButton("Insert field")
        fields_layout.addWidget(self.insert_field_button)
        tokens_row.addWidget(fields_box, 1)

        geometry_box = QWidget()
        geometry_layout = QVBoxLayout(geometry_box)
        geometry_layout.addWidget(QLabel("Geometry tokens"))
        self.geometry_list = QListWidget()
        for label, token in _GEOMETRY_TOKENS:
            item = QListWidgetItem(label)
            item.setData(getattr(Qt, "UserRole", 32), token)
            self.geometry_list.addItem(item)
        geometry_layout.addWidget(self.geometry_list)
        self.insert_geometry_button = QPushButton("Insert token")
        geometry_layout.addWidget(self.insert_geometry_button)
        tokens_row.addWidget(geometry_box, 1)

        snippet_box = QWidget()
        snippet_layout = QVBoxLayout(snippet_box)
        snippet_layout.addWidget(QLabel("Snippets"))
        self.snippet_category_combo = QComboBox()
        self.snippet_category_combo.addItem("All")
        for category in _SNIPPET_LIBRARY.keys():
            self.snippet_category_combo.addItem(category)
        snippet_layout.addWidget(self.snippet_category_combo)

        self.snippet_search_edit = QLineEdit()
        self.snippet_search_edit.setPlaceholderText("Filter snippets (name, category, or content)")
        snippet_layout.addWidget(self.snippet_search_edit)

        self.snippet_list = QListWidget()
        snippet_layout.addWidget(self.snippet_list)

        self.snippet_preview = QTextEdit()
        self.snippet_preview.setReadOnly(True)
        self.snippet_preview.setPlaceholderText("Select a snippet to inspect it before applying.")
        snippet_layout.addWidget(self.snippet_preview)

        snippet_actions = QHBoxLayout()
        self.insert_snippet_button = QPushButton("Insert")
        self.replace_with_snippet_button = QPushButton("Replace")
        snippet_actions.addWidget(self.insert_snippet_button)
        snippet_actions.addWidget(self.replace_with_snippet_button)
        snippet_layout.addLayout(snippet_actions)

        tokens_row.addWidget(snippet_box, 1)

        preview_header = QHBoxLayout()
        root.addLayout(preview_header)
        preview_header.addWidget(QLabel("Preview"))
        self.status_label = QLabel("Select a layer, define a target field, and preview the expression.")
        if hasattr(self.status_label, "setWordWrap"):
            self.status_label.setWordWrap(True)
        preview_header.addWidget(self.status_label, 1)

        self.preview_table = QTableWidget(0, 5)
        self.preview_table.setHorizontalHeaderLabels(
            ["FID", "Will update", "Original", "Computed", "Result"]
        )
        header = getattr(self.preview_table, "horizontalHeader", None)
        if callable(header):
            header_obj = header()
            stretch = getattr(header_obj, "setStretchLastSection", None)
            if callable(stretch):
                stretch(True)
        root.addWidget(self.preview_table, 3)

        button_row = QHBoxLayout()
        root.addLayout(button_row)
        self.preview_button = QPushButton("Preview")
        button_row.addWidget(self.preview_button)
        self.open_processing_button = QPushButton("Open Processing Dialog")
        button_row.addWidget(self.open_processing_button)
        self.cancel_button = QPushButton("Cancel")
        button_row.addWidget(self.cancel_button)
        button_row.addStretch(1)

        self.refresh_layers_button.clicked.connect(self._refresh_layers)
        self.layer_combo.currentIndexChanged.connect(self._on_layer_changed)
        self.insert_field_button.clicked.connect(self._insert_selected_field)
        self.insert_geometry_button.clicked.connect(self._insert_selected_geometry_token)
        self.fields_list.itemDoubleClicked.connect(lambda _item: self._insert_selected_field())
        self.geometry_list.itemDoubleClicked.connect(lambda _item: self._insert_selected_geometry_token())
        self.load_preset_button.clicked.connect(self._load_selected_preset)
        self.preset_combo.currentIndexChanged.connect(self._load_selected_preset)
        self.snippet_category_combo.currentIndexChanged.connect(self._populate_snippet_list)
        self.snippet_search_edit.textChanged.connect(self._populate_snippet_list)
        self.snippet_list.currentItemChanged.connect(self._on_snippet_selection_changed)
        self.snippet_list.itemDoubleClicked.connect(lambda _item: self._apply_selected_snippet(replace=False))
        self.insert_snippet_button.clicked.connect(lambda: self._apply_selected_snippet(replace=False))
        self.replace_with_snippet_button.clicked.connect(lambda: self._apply_selected_snippet(replace=True))
        self.preview_button.clicked.connect(self._run_preview)
        self.open_processing_button.clicked.connect(self._accept_and_open_processing)
        self.cancel_button.clicked.connect(self.reject)

        self._populate_snippet_list()

    def _vector_layers(self) -> list[Any]:
        if QgsProject is None:
            return []
        project_instance = getattr(QgsProject, "instance", None)
        project = project_instance() if callable(project_instance) else None
        layer_map = getattr(project, "mapLayers", None)
        layers = layer_map().values() if callable(layer_map) else []
        result = []
        for layer in layers:
            fields_getter = getattr(layer, "fields", None)
            source_getter = getattr(layer, "source", None)
            if not callable(fields_getter) or not callable(source_getter):
                continue
            try:
                fields = fields_getter()
                source = str(source_getter() or "").strip()
            except Exception:
                continue
            if fields is None:
                continue
            if not source and not hasattr(layer, "dataProvider"):
                continue
            result.append(layer)
        result.sort(key=lambda lyr: str(getattr(lyr, "name", lambda: "")()).lower())
        return result

    def _refresh_layers(self) -> None:
        self.layer_combo.clear()
        self._layer_by_index = {}
        for index, layer in enumerate(self._vector_layers()):
            name_getter = getattr(layer, "name", None)
            name = str(name_getter() if callable(name_getter) else f"Layer {index + 1}")
            self.layer_combo.addItem(name)
            self._layer_by_index[index] = layer
        self._on_layer_changed()

    def _select_active_layer(self) -> None:
        active_layer_getter = getattr(self.iface, "activeLayer", None)
        active_layer = active_layer_getter() if callable(active_layer_getter) else None
        if active_layer is None:
            return
        for index, layer in self._layer_by_index.items():
            if layer == active_layer:
                self.layer_combo.setCurrentIndex(index)
                self._on_layer_changed()
                return

    def _current_layer(self):
        return self._layer_by_index.get(self.layer_combo.currentIndex())

    def _field_names(self, layer) -> list[str]:
        if layer is None:
            return []
        fields_getter = getattr(layer, "fields", None)
        fields = fields_getter() if callable(fields_getter) else None
        if fields is None:
            return []
        names: list[str] = []
        try:
            for field in fields:
                name_getter = getattr(field, "name", None)
                if callable(name_getter):
                    name = str(name_getter() or "").strip()
                else:
                    name = str(field).strip()
                if name:
                    names.append(name)
        except Exception:
            pass
        return names

    def _on_layer_changed(self, *_args) -> None:
        layer = self._current_layer()
        self.fields_list.clear()
        field_names = self._field_names(layer)
        for name in field_names:
            self.fields_list.addItem(name)
        if field_names and not self.field_name_edit.text().strip():
            self.field_name_edit.setText(field_names[0])

    def _insert_text(self, text: str) -> None:
        value = str(text or "")
        if not value:
            return
        cursor = self.expression_edit.textCursor()
        cursor.insertText(value)
        self.expression_edit.setFocus()

    def _insert_selected_field(self) -> None:
        item = self.fields_list.currentItem()
        if item is None:
            return
        self._insert_text(item.text())

    def _insert_selected_geometry_token(self) -> None:
        item = self.geometry_list.currentItem()
        if item is None:
            return
        token = item.data(getattr(Qt, "UserRole", 32)) if Qt is not None else None
        self._insert_text(str(token or item.text()))

    def _load_selected_preset(self, *_args) -> None:
        index = self.preset_combo.currentIndex()
        if index <= 0 or index > len(_PRESET_EXPRESSIONS):
            return
        _label, expression = _PRESET_EXPRESSIONS[index - 1]
        self.expression_edit.setPlainText(expression)

    def _snippet_items(self) -> list[tuple[str, str, str]]:
        category = self.snippet_category_combo.currentText().strip()
        search_text = self.snippet_search_edit.text().strip().lower()
        items: list[tuple[str, str, str]] = []
        for cat_name, snippets in _SNIPPET_LIBRARY.items():
            if category and category != "All" and category != cat_name:
                continue
            for label, snippet in snippets:
                if search_text:
                    haystack = f"{cat_name} {label} {snippet}".lower()
                    if search_text not in haystack:
                        continue
                items.append((cat_name, label, snippet))
        return items

    def _populate_snippet_list(self, *_args) -> None:
        self.snippet_list.clear()
        count = 0
        for category, label, snippet in self._snippet_items():
            item = QListWidgetItem(f"{label} ({category})")
            item.setData(getattr(Qt, "UserRole", 32), snippet)
            self.snippet_list.addItem(item)
            count += 1
        self.snippet_preview.clear()
        if count == 0:
            self.snippet_preview.setPlainText("No snippets match the current filter.")

    def _on_snippet_selection_changed(self, current, _previous) -> None:
        if current is None:
            self.snippet_preview.clear()
            return
        snippet = current.data(getattr(Qt, "UserRole", 32)) if Qt is not None else None
        self.snippet_preview.setPlainText(str(snippet or ""))

    def _apply_selected_snippet(self, *, replace: bool) -> None:
        item = self.snippet_list.currentItem()
        if item is None:
            return
        snippet = item.data(getattr(Qt, "UserRole", 32)) if Qt is not None else None
        snippet_text = str(snippet or "").strip()
        if not snippet_text:
            return
        if replace:
            self.expression_edit.setPlainText(snippet_text)
            self.expression_edit.setFocus()
            return
        self._insert_text(snippet_text)

    def _validate_inputs(self) -> str:
        if self._current_layer() is None:
            return "Choose an input vector layer."
        if not self.field_name_edit.text().strip():
            return "Enter a target field name."
        if not self.expression_edit.toPlainText().strip():
            return "Enter an expression to evaluate."
        return ""

    def _build_runtime_args(self, *, preview_only: bool) -> dict[str, Any]:
        layer = self._current_layer()
        source_getter = getattr(layer, "source", None)
        source = source_getter() if callable(source_getter) else ""
        input_path = _materialize_vector_input_source(layer, source, feedback=None)
        args: dict[str, Any] = {
            "input": input_path,
            "field": self.field_name_edit.text().strip(),
            "field_type": self.field_type_combo.currentText().strip() or "float",
            "expression": self.expression_edit.toPlainText().strip(),
            "overwrite": bool(self.overwrite_checkbox.isChecked()),
        }
        if preview_only:
            args["preview_rows"] = int(self.preview_rows_spin.value())
        return args

    def _run_preview(self) -> None:
        problem = self._validate_inputs()
        if problem:
            self._set_status(problem, is_error=True)
            return

        self.preview_button.setEnabled(False)
        try:
            session = create_runtime_session(include_pro=self.include_pro, tier=self.tier)
            response_raw = session.run_tool_json_stream(
                "field_calculator",
                json.dumps(self._build_runtime_args(preview_only=True)),
                lambda _evt: None,
            )
            response = json.loads(response_raw) if isinstance(response_raw, str) else response_raw
        except RuntimeBootstrapError as exc:
            self._set_status(str(exc), is_error=True)
            return
        except Exception as exc:
            self._set_status(f"Preview failed: {exc}", is_error=True)
            return
        finally:
            self.preview_button.setEnabled(True)

        outputs = response.get("outputs") if isinstance(response, dict) else None
        if not isinstance(outputs, dict):
            error_message = "Preview did not return a valid payload."
            if isinstance(response, dict):
                for key in ("error", "message", "detail", "details", "reason"):
                    value = response.get(key)
                    if isinstance(value, str) and value.strip():
                        error_message = value.strip()
                        break
            self._set_status(error_message, is_error=True)
            return

        preview_rows = outputs.get("preview")
        if not isinstance(preview_rows, list):
            self._set_status("Preview response did not contain preview rows.", is_error=True)
            return

        self.preview_table.setRowCount(len(preview_rows))
        for row_index, row in enumerate(preview_rows):
            row_data = row if isinstance(row, dict) else {}
            values = [
                row_data.get("fid"),
                row_data.get("will_update"),
                row_data.get("original_value"),
                row_data.get("computed_value"),
                row_data.get("result_value"),
            ]
            for col_index, value in enumerate(values):
                item = QTableWidgetItem(self._format_preview_value(value))
                self.preview_table.setItem(row_index, col_index, item)

        normalized_expression = str(outputs.get("normalized_expression", "")).strip()
        normalized_where = str(outputs.get("normalized_where", "")).strip()
        detail = f"Previewed {len(preview_rows)} row(s)."
        if normalized_expression:
            detail += f" Normalized expression: {normalized_expression}."
        if normalized_where:
            detail += f" Normalized WHERE: {normalized_where}."
        self._set_status(detail, is_error=False)

    def _format_preview_value(self, value: Any) -> str:
        if value is None:
            return "NULL"
        if isinstance(value, bool):
            return "true" if value else "false"
        if isinstance(value, (int, float)):
            return str(value)
        if isinstance(value, str):
            return value
        try:
            return json.dumps(value, ensure_ascii=True)
        except Exception:
            return str(value)

    def _set_status(self, text: str, *, is_error: bool) -> None:
        self.status_label.setText(text)
        style = "color:#b03a2e;" if is_error else "color:#1e8449;"
        set_style = getattr(self.status_label, "setStyleSheet", None)
        if callable(set_style):
            set_style(style)

    def _accept_and_open_processing(self) -> None:
        problem = self._validate_inputs()
        if problem:
            self._set_status(problem, is_error=True)
            return
        self._launch_params = {
            "input": self._current_layer(),
            "field": self.field_name_edit.text().strip(),
            "field_type": self.field_type_combo.currentText().strip() or "float",
            "expression": self.expression_edit.toPlainText().strip(),
            "overwrite": bool(self.overwrite_checkbox.isChecked()),
        }
        self.accept()


def run_field_calculator_assistant(iface, include_pro: bool = True, tier: str = "open") -> dict[str, Any] | None:
    dialog = FieldCalculatorAssistantDialog(
        iface,
        include_pro=include_pro,
        tier=tier,
        parent=iface.mainWindow() if iface is not None else None,
    )
    exec_fn = getattr(dialog, "exec", None) or getattr(dialog, "exec_", None)
    if not callable(exec_fn):
        return None
    result = exec_fn()
    accepted = bool(result)
    if not accepted:
        return {}
    return dialog.launch_params() or {}
