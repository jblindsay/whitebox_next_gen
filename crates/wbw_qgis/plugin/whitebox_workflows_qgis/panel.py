from __future__ import annotations

from typing import Any

try:
    from qgis.PyQt.QtWidgets import (
        QDockWidget,
        QLabel,
        QLineEdit,
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
        self._matches_label = QLabel("Matches: 0")

        self._refresh_button = QPushButton("Refresh Catalog + Help")
        self._diagnostics_button = QPushButton("Runtime Diagnostics")

        layout.addWidget(self._status_label)
        layout.addWidget(self._tier_label)
        layout.addWidget(self._catalog_label)
        layout.addWidget(self._version_label)
        layout.addWidget(self._search_label)
        layout.addWidget(self._search_box)
        layout.addWidget(self._matches_label)
        layout.addWidget(self._refresh_button)
        layout.addWidget(self._diagnostics_button)

        container.setLayout(layout)
        self.setWidget(container)

        self._catalog: list[dict[str, Any]] = []
        self._search_box.textChanged.connect(self._on_search_text_changed)

    def on_refresh(self, callback):
        self._refresh_button.clicked.connect(callback)

    def on_diagnostics(self, callback):
        self._diagnostics_button.clicked.connect(callback)

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
        self._update_match_count(self._search_box.text())

    def _on_search_text_changed(self, text: str) -> None:
        self._update_match_count(text)

    def _update_match_count(self, text: str) -> None:
        query = text.strip().lower()
        if not query:
            self._matches_label.setText(f"Matches: {len(self._catalog)}")
            return

        matches = 0
        for item in self._catalog:
            haystack = " ".join(
                [
                    str(item.get("id", "")),
                    str(item.get("display_name", "")),
                    str(item.get("category", "")),
                    str(item.get("summary", "")),
                ]
            ).lower()
            if query in haystack:
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
