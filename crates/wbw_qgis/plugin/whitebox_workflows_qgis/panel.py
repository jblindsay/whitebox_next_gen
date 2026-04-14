from __future__ import annotations

from typing import Any

try:
    from qgis.PyQt.QtWidgets import (
        QCheckBox,
        QDockWidget,
        QLabel,
        QLineEdit,
        QListWidget,
        QListWidgetItem,
        QPushButton,
        QVBoxLayout,
        QWidget,
    )
except Exception:  # pragma: no cover
    class _DummySignal:  # type: ignore[override]
        def connect(self, *_args, **_kwargs):
            return None

    class _DummyWidget:  # type: ignore[override]
        def __init__(self, *_args, **_kwargs):
            pass

    class QLabel(_DummyWidget):  # type: ignore[override]
        def setText(self, *_args, **_kwargs):
            return None

    class QPushButton(_DummyWidget):  # type: ignore[override]
        def __init__(self, *_args, **_kwargs):
            self.clicked = _DummySignal()

    class QCheckBox(_DummyWidget):  # type: ignore[override]
        def __init__(self, *_args, **_kwargs):
            self.stateChanged = _DummySignal()

        def isChecked(self):
            return True

        def setChecked(self, *_args, **_kwargs):
            return None

    class QLineEdit(_DummyWidget):  # type: ignore[override]
        def __init__(self, *_args, **_kwargs):
            self.textChanged = _DummySignal()

        def text(self):
            return ""

        def setPlaceholderText(self, *_args, **_kwargs):
            return None

    class QVBoxLayout(_DummyWidget):  # type: ignore[override]
        def addWidget(self, *_args, **_kwargs):
            return None

    class QListWidget(_DummyWidget):  # type: ignore[override]
        def __init__(self, *_args, **_kwargs):
            self.itemDoubleClicked = _DummySignal()

        def clear(self):
            return None

        def addItem(self, *_args, **_kwargs):
            return None

        def row(self, *_args, **_kwargs):
            return -1

    class QListWidgetItem(_DummyWidget):  # type: ignore[override]
        pass

    class QWidget(_DummyWidget):  # type: ignore[override]
        def setLayout(self, *_args, **_kwargs):
            return None

    class QDockWidget(_DummyWidget):  # type: ignore[override]
        def setObjectName(self, *_args, **_kwargs):
            return None

        def setWidget(self, *_args, **_kwargs):
            return None


class WhiteboxDockPanel(QDockWidget):
    def __init__(self, parent=None):
        super().__init__("Whitebox Workflows", parent)
        self.setObjectName("WhiteboxWorkflowsDock")

        container = QWidget(self)
        layout = QVBoxLayout(container)

        self._status_label = QLabel("Status: unknown")
        self._tier_label = QLabel("Tier: unknown")
        self._catalog_label = QLabel("Catalog: unknown")
        self._version_label = QLabel("QGIS: unknown")
        self._search_label = QLabel("Tool Search")
        self._search_box = QLineEdit()
        self._search_box.setPlaceholderText("Search by tool id, name, category, or summary")
        self._show_available_checkbox = QCheckBox("Show available")
        self._show_available_checkbox.setChecked(True)
        self._show_locked_checkbox = QCheckBox("Show locked")
        self._show_locked_checkbox.setChecked(True)
        self._matches_label = QLabel("Matches: 0")
        self._results_list = QListWidget()

        self._refresh_button = QPushButton("Refresh Catalog + Help")
        self._diagnostics_button = QPushButton("Runtime Diagnostics")

        layout.addWidget(self._status_label)
        layout.addWidget(self._tier_label)
        layout.addWidget(self._catalog_label)
        layout.addWidget(self._version_label)
        layout.addWidget(self._search_label)
        layout.addWidget(self._search_box)
        layout.addWidget(self._show_available_checkbox)
        layout.addWidget(self._show_locked_checkbox)
        layout.addWidget(self._matches_label)
        layout.addWidget(self._results_list)
        layout.addWidget(self._refresh_button)
        layout.addWidget(self._diagnostics_button)

        container.setLayout(layout)
        self.setWidget(container)

        self._catalog: list[dict[str, Any]] = []
        self._filtered_tool_ids: list[str] = []
        self._search_box.textChanged.connect(self._on_search_text_changed)
        self._show_available_checkbox.stateChanged.connect(self._on_filter_changed)
        self._show_locked_checkbox.stateChanged.connect(self._on_filter_changed)

    def on_refresh(self, callback):
        self._refresh_button.clicked.connect(callback)

    def on_diagnostics(self, callback):
        self._diagnostics_button.clicked.connect(callback)

    def on_open_tool(self, callback):
        def _open(item):
            row = self._results_list.row(item)
            if row < 0 or row >= len(self._filtered_tool_ids):
                return
            callback(self._filtered_tool_ids[row])

        self._results_list.itemDoubleClicked.connect(_open)

    def update_state(
        self,
        *,
        status: str,
        requested_tier: str,
        effective_tier: str,
        available_count: int,
        locked_count: int,
        qgis_version: str,
    ) -> None:
        self._status_label.setText(f"Status: {status}")
        self._tier_label.setText(
            f"Tier: requested={requested_tier}, effective={effective_tier}"
        )
        self._catalog_label.setText(
            f"Catalog: available={available_count}, locked={locked_count}"
        )
        self._version_label.setText(f"QGIS: {qgis_version or 'unknown'}")

    def set_catalog(self, catalog: list[dict[str, Any]]) -> None:
        self._catalog = list(catalog)
        self._refresh_results(self._search_box.text())

    def _on_search_text_changed(self, text: str) -> None:
        self._refresh_results(text)

    def _on_filter_changed(self, _value: int) -> None:
        self._refresh_results(self._search_box.text())

    def _refresh_results(self, text: str) -> None:
        query = text.strip().lower()

        show_available = bool(self._show_available_checkbox.isChecked())
        show_locked = bool(self._show_locked_checkbox.isChecked())

        self._results_list.clear()
        self._filtered_tool_ids = []

        matches = 0
        for item in self._catalog:
            is_locked = bool(item.get("locked", False))
            if is_locked and not show_locked:
                continue
            if (not is_locked) and not show_available:
                continue

            haystack = " ".join(
                [
                    str(item.get("id", "")),
                    str(item.get("display_name", "")),
                    str(item.get("category", "")),
                    str(item.get("summary", "")),
                ]
            ).lower()
            if query and query not in haystack:
                continue

            tool_id = str(item.get("id", ""))
            display_name = str(item.get("display_name", tool_id))
            category = str(item.get("category", "General"))
            badge = "[LOCKED] " if is_locked else ""
            label = f"{badge}{display_name} ({tool_id}) — {category}"

            self._results_list.addItem(QListWidgetItem(label))
            self._filtered_tool_ids.append(tool_id)
            matches += 1

        self._matches_label.setText(f"Matches: {matches}")


def summarize_catalog(catalog: list[dict[str, Any]]) -> tuple[int, int]:
    available = 0
    locked = 0
    for item in catalog:
        if bool(item.get("locked", False)):
            locked += 1
        else:
            available += 1
    return available, locked
