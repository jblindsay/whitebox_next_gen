from __future__ import annotations

from typing import Any

try:
    from qgis.PyQt.QtCore import QEvent, Qt
    from qgis.PyQt.QtGui import QKeySequence, QShortcut
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
    class _QtShim:  # type: ignore[override]
        CustomContextMenu = 0

    Qt = _QtShim()  # type: ignore[assignment]

    class _QEventShim:  # type: ignore[override]
        FocusIn = 8

    QEvent = _QEventShim()  # type: ignore[assignment]

    class _DummySignal:  # type: ignore[override]
        def connect(self, *_args, **_kwargs):
            return None

    class QKeySequence:  # type: ignore[override]
        def __init__(self, *_args, **_kwargs):
            pass

    class QShortcut:  # type: ignore[override]
        def __init__(self, *_args, **_kwargs):
            self.activated = _DummySignal()

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
            self.returnPressed = _DummySignal()

        def text(self):
            return ""

        def setPlaceholderText(self, *_args, **_kwargs):
            return None

        def setFocus(self, *_args, **_kwargs):
            return None

        def selectAll(self, *_args, **_kwargs):
            return None

        def clear(self, *_args, **_kwargs):
            return None

        def setText(self, *_args, **_kwargs):
            return None

    class QVBoxLayout(_DummyWidget):  # type: ignore[override]
        def addWidget(self, *_args, **_kwargs):
            return None

    class QListWidget(_DummyWidget):  # type: ignore[override]
        def __init__(self, *_args, **_kwargs):
            self.itemDoubleClicked = _DummySignal()
            self.customContextMenuRequested = _DummySignal()

        def clear(self):
            return None

        def addItem(self, *_args, **_kwargs):
            return None

        def row(self, *_args, **_kwargs):
            return -1

        def currentRow(self):
            return -1

        def setCurrentRow(self, *_args, **_kwargs):
            return None

        def setFocus(self, *_args, **_kwargs):
            return None

        def count(self):
            return 0

        def setContextMenuPolicy(self, *_args, **_kwargs):
            return None

        def mapToGlobal(self, pos):
            return pos

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
        self._quick_open_checkbox = QCheckBox("Quick-open top match on Enter")
        self._quick_open_checkbox.setChecked(True)
        self._show_available_checkbox = QCheckBox("Show available")
        self._show_available_checkbox.setChecked(True)
        self._show_locked_checkbox = QCheckBox("Show locked")
        self._show_locked_checkbox.setChecked(True)
        self._matches_label = QLabel("Matches: 0")
        self._results_list = QListWidget()

        self._favorites_label = QLabel("Favorite Tools")
        self._favorites_list = QListWidget()
        self._favorite_add_button = QPushButton("Add Selected to Favorites")
        self._favorite_remove_button = QPushButton("Remove Selected Favorite")
        self._favorite_up_button = QPushButton("Move Favorite Up")
        self._favorite_down_button = QPushButton("Move Favorite Down")
        self._favorite_clear_button = QPushButton("Clear Favorites")

        self._recent_label = QLabel("Recent Tools")
        self._recent_list = QListWidget()
        self._recent_clear_button = QPushButton("Clear Recents")
        self._shortcut_hint_label = QLabel(
            "Shortcuts: /=focus search, Esc=clear search, Up/Down=jump to results, Enter=open result, Ctrl/Cmd+D=toggle favorite, Delete/Backspace=remove favorite"
        )

        self._refresh_button = QPushButton("Refresh Catalog + Help")
        self._diagnostics_button = QPushButton("Runtime Diagnostics")

        layout.addWidget(self._status_label)
        layout.addWidget(self._tier_label)
        layout.addWidget(self._catalog_label)
        layout.addWidget(self._version_label)
        layout.addWidget(self._search_label)
        layout.addWidget(self._search_box)
        layout.addWidget(self._quick_open_checkbox)
        layout.addWidget(self._show_available_checkbox)
        layout.addWidget(self._show_locked_checkbox)
        layout.addWidget(self._matches_label)
        layout.addWidget(self._results_list)
        layout.addWidget(self._favorites_label)
        layout.addWidget(self._favorites_list)
        layout.addWidget(self._favorite_add_button)
        layout.addWidget(self._favorite_remove_button)
        layout.addWidget(self._favorite_up_button)
        layout.addWidget(self._favorite_down_button)
        layout.addWidget(self._favorite_clear_button)
        layout.addWidget(self._recent_label)
        layout.addWidget(self._recent_list)
        layout.addWidget(self._recent_clear_button)
        layout.addWidget(self._shortcut_hint_label)
        layout.addWidget(self._refresh_button)
        layout.addWidget(self._diagnostics_button)

        container.setLayout(layout)
        self.setWidget(container)

        self._catalog: list[dict[str, Any]] = []
        self._filtered_tool_ids: list[str] = []
        self._favorite_tool_ids: list[str] = []
        self._favorite_display_ids: list[str] = []
        self._recent_tool_ids: list[str] = []
        self._search_box.textChanged.connect(self._on_search_text_changed)
        self._search_box.returnPressed.connect(self._open_quick_match)
        self._show_available_checkbox.stateChanged.connect(self._on_filter_changed)
        self._show_locked_checkbox.stateChanged.connect(self._on_filter_changed)

        self._search_box.installEventFilter(self)
        self._results_list.installEventFilter(self)
        self._favorites_list.installEventFilter(self)
        self._recent_list.installEventFilter(self)

        self._results_list.setContextMenuPolicy(Qt.CustomContextMenu)
        self._results_list.customContextMenuRequested.connect(self._on_results_context_menu)
        self._favorites_list.setContextMenuPolicy(Qt.CustomContextMenu)
        self._favorites_list.customContextMenuRequested.connect(self._on_favorites_context_menu)

        self._open_tool_callback = None
        self._tool_context_menu_callback = None

        # Keyboard accelerators for high-frequency workflows.
        self._shortcut_open_result = QShortcut(QKeySequence("Return"), self)
        self._shortcut_open_result_alt = QShortcut(QKeySequence("Enter"), self)
        self._shortcut_open_result_list = QShortcut(QKeySequence("Return"), self._results_list)
        self._shortcut_open_result_alt_list = QShortcut(QKeySequence("Enter"), self._results_list)
        self._shortcut_focus_search_slash = QShortcut(QKeySequence("/"), self)
        self._shortcut_focus_search_ctrlf = QShortcut(QKeySequence("Ctrl+F"), self)
        self._shortcut_focus_search_metaf = QShortcut(QKeySequence("Meta+F"), self)
        self._shortcut_focus_results_down = QShortcut(QKeySequence("Down"), self._search_box)
        self._shortcut_focus_results_up = QShortcut(QKeySequence("Up"), self._search_box)
        self._shortcut_clear_search_esc = QShortcut(QKeySequence("Esc"), self)
        self._shortcut_toggle_favorite_ctrl = QShortcut(QKeySequence("Ctrl+D"), self)
        self._shortcut_toggle_favorite_meta = QShortcut(QKeySequence("Meta+D"), self)
        self._shortcut_remove_favorite_del = QShortcut(QKeySequence("Delete"), self)
        self._shortcut_remove_favorite_back = QShortcut(QKeySequence("Backspace"), self)

        self._shortcut_open_result.activated.connect(self._open_selected_result)
        self._shortcut_open_result_alt.activated.connect(self._open_selected_result)
        self._shortcut_open_result_list.activated.connect(self._open_selected_result)
        self._shortcut_open_result_alt_list.activated.connect(self._open_selected_result)
        self._shortcut_focus_search_slash.activated.connect(self._focus_search_box)
        self._shortcut_focus_search_ctrlf.activated.connect(self._focus_search_box)
        self._shortcut_focus_search_metaf.activated.connect(self._focus_search_box)
        self._shortcut_focus_results_down.activated.connect(self._focus_results_first)
        self._shortcut_focus_results_up.activated.connect(self._focus_results_last)
        self._shortcut_clear_search_esc.activated.connect(self._clear_search_box)
        self._shortcut_toggle_favorite_ctrl.activated.connect(self._toggle_selected_favorite)
        self._shortcut_toggle_favorite_meta.activated.connect(self._toggle_selected_favorite)
        self._shortcut_remove_favorite_del.activated.connect(self._remove_selected_favorite_shortcut)
        self._shortcut_remove_favorite_back.activated.connect(self._remove_selected_favorite_shortcut)

        self._toggle_favorite_callback = None
        self._remove_favorite_shortcut_callback = None
        self._filter_state_callback = None
        self._search_state_callback = None
        self._focus_area_callback = None
        self._focus_area = "search"

    def on_refresh(self, callback):
        self._refresh_button.clicked.connect(callback)

    def on_diagnostics(self, callback):
        self._diagnostics_button.clicked.connect(callback)

    def on_open_tool(self, callback):
        self._open_tool_callback = callback

        def _open(item):
            row = self._results_list.row(item)
            if row < 0 or row >= len(self._filtered_tool_ids):
                return
            callback(self._filtered_tool_ids[row])

        self._results_list.itemDoubleClicked.connect(_open)

    def on_open_recent_tool(self, callback):
        def _open_recent(item):
            row = self._recent_list.row(item)
            if row < 0 or row >= len(self._recent_tool_ids):
                return
            callback(self._recent_tool_ids[row])

        self._recent_list.itemDoubleClicked.connect(_open_recent)

    def on_open_favorite_tool(self, callback):
        def _open_favorite(item):
            row = self._favorites_list.row(item)
            if row < 0 or row >= len(self._favorite_display_ids):
                return
            callback(self._favorite_display_ids[row])

        self._favorites_list.itemDoubleClicked.connect(_open_favorite)

    def on_add_favorite(self, callback):
        self._favorite_add_button.clicked.connect(callback)

    def on_remove_favorite(self, callback):
        self._favorite_remove_button.clicked.connect(callback)

    def on_toggle_selected_favorite(self, callback):
        self._toggle_favorite_callback = callback

    def on_remove_selected_favorite_shortcut(self, callback):
        self._remove_favorite_shortcut_callback = callback

    def on_move_favorite_up(self, callback):
        self._favorite_up_button.clicked.connect(callback)

    def on_move_favorite_down(self, callback):
        self._favorite_down_button.clicked.connect(callback)

    def on_clear_favorites(self, callback):
        self._favorite_clear_button.clicked.connect(callback)

    def on_clear_recents(self, callback):
        self._recent_clear_button.clicked.connect(callback)

    def on_quick_open_toggled(self, callback):
        self._quick_open_checkbox.stateChanged.connect(callback)

    def on_filter_state_changed(self, callback):
        self._filter_state_callback = callback

    def on_search_state_changed(self, callback):
        self._search_state_callback = callback

    def on_focus_area_changed(self, callback):
        self._focus_area_callback = callback

    def on_tool_context_menu(self, callback):
        self._tool_context_menu_callback = callback

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
        self._refresh_favorites_list()

    def set_favorites(self, tool_ids: list[str]) -> None:
        self._favorite_tool_ids = list(tool_ids)
        self._refresh_results(self._search_box.text())
        self._refresh_favorites_list()

    def selected_result_tool_id(self) -> str:
        row = self._results_list.currentRow()
        if row < 0 or row >= len(self._filtered_tool_ids):
            return ""
        return self._filtered_tool_ids[row]

    def quick_open_enabled(self) -> bool:
        return bool(self._quick_open_checkbox.isChecked())

    def set_quick_open_enabled(self, enabled: bool) -> None:
        self._quick_open_checkbox.setChecked(bool(enabled))

    def show_available_enabled(self) -> bool:
        return bool(self._show_available_checkbox.isChecked())

    def set_show_available_enabled(self, enabled: bool) -> None:
        self._show_available_checkbox.setChecked(bool(enabled))

    def show_locked_enabled(self) -> bool:
        return bool(self._show_locked_checkbox.isChecked())

    def set_show_locked_enabled(self, enabled: bool) -> None:
        self._show_locked_checkbox.setChecked(bool(enabled))

    def search_text(self) -> str:
        return str(self._search_box.text())

    def set_search_text(self, text: str) -> None:
        self._search_box.setText(str(text))

    def focus_area(self) -> str:
        return str(self._focus_area)

    def set_focus_area(self, area: str) -> None:
        target = str(area).strip().lower()
        if target == "results":
            self._focus_results_first()
            return
        if target == "favorites":
            self._favorites_list.setFocus()
            return
        if target == "recents":
            self._recent_list.setFocus()
            return
        self._focus_search_box()

    def eventFilter(self, obj, event):  # type: ignore[override]
        if event is not None and event.type() == QEvent.FocusIn:
            area = self._focus_area_name_for_obj(obj)
            if area:
                self._focus_area = area
                if self._focus_area_callback is not None:
                    self._focus_area_callback()
        return super().eventFilter(obj, event)

    def _focus_area_name_for_obj(self, obj) -> str:
        if obj is self._search_box:
            return "search"
        if obj is self._results_list:
            return "results"
        if obj is self._favorites_list:
            return "favorites"
        if obj is self._recent_list:
            return "recents"
        return ""

    def top_result_tool_id(self) -> str:
        if not self._filtered_tool_ids:
            return ""
        return self._filtered_tool_ids[0]

    def selected_favorite_tool_id(self) -> str:
        row = self._favorites_list.currentRow()
        if row < 0 or row >= len(self._favorite_display_ids):
            return ""
        return self._favorite_display_ids[row]

    def selected_favorite_index(self) -> int:
        row = self._favorites_list.currentRow()
        if row < 0 or row >= len(self._favorite_display_ids):
            return -1
        return row

    def is_favorite(self, tool_id: str) -> bool:
        return tool_id in self._favorite_tool_ids

    def select_favorite_index(self, index: int) -> None:
        if index < 0 or index >= len(self._favorite_display_ids):
            return
        self._favorites_list.setCurrentRow(index)

    def set_recent_tools(self, tool_ids: list[str]) -> None:
        self._recent_tool_ids = list(tool_ids)
        self._recent_list.clear()
        for tool_id in self._recent_tool_ids:
            self._recent_list.addItem(QListWidgetItem(tool_id))

    def _refresh_favorites_list(self) -> None:
        self._favorites_list.clear()
        self._favorite_display_ids = []

        catalog_by_id = {str(item.get("id", "")): item for item in self._catalog}
        for tool_id in self._favorite_tool_ids:
            item = catalog_by_id.get(tool_id)
            if item is None:
                label = tool_id
            else:
                display_name = str(item.get("display_name", tool_id))
                is_locked = bool(item.get("locked", False))
                badge = "[LOCKED] " if is_locked else ""
                label = f"{badge}{display_name} ({tool_id})"
            self._favorites_list.addItem(QListWidgetItem(label))
            self._favorite_display_ids.append(tool_id)

    def _on_search_text_changed(self, text: str) -> None:
        self._refresh_results(text)
        if self._search_state_callback is not None:
            self._search_state_callback()

    def _on_filter_changed(self, _value: int) -> None:
        self._refresh_results(self._search_box.text())
        if self._filter_state_callback is not None:
            self._filter_state_callback()

    def _open_selected_result(self) -> None:
        callback = getattr(self, "_open_tool_callback", None)
        if callback is None:
            return
        tool_id = self.selected_result_tool_id()
        if tool_id:
            callback(tool_id)

    def _toggle_selected_favorite(self) -> None:
        if self._toggle_favorite_callback is None:
            return
        self._toggle_favorite_callback()

    def _remove_selected_favorite_shortcut(self) -> None:
        if self._remove_favorite_shortcut_callback is None:
            return
        self._remove_favorite_shortcut_callback()

    def _focus_search_box(self) -> None:
        self._focus_area = "search"
        self._search_box.setFocus()
        self._search_box.selectAll()

    def _clear_search_box(self) -> None:
        self._search_box.clear()

    def _focus_results_first(self) -> None:
        self._focus_results_index(0)

    def _focus_results_last(self) -> None:
        self._focus_results_index(len(self._filtered_tool_ids) - 1)

    def _focus_results_index(self, index: int) -> None:
        # Use filtered tool ids count to avoid selecting empty-state hint rows.
        if not self._filtered_tool_ids:
            return
        if index < 0:
            index = 0
        if index >= len(self._filtered_tool_ids):
            index = len(self._filtered_tool_ids) - 1
        self._focus_area = "results"
        self._results_list.setCurrentRow(index)
        self._results_list.setFocus()

    def _open_quick_match(self) -> None:
        if not self._quick_open_checkbox.isChecked():
            return
        callback = getattr(self, "_open_tool_callback", None)
        if callback is None:
            return
        tool_id = self.top_result_tool_id()
        if tool_id:
            callback(tool_id)

    def _on_results_context_menu(self, pos) -> None:
        if self._tool_context_menu_callback is None:
            return
        tool_id = self.selected_result_tool_id()
        if not tool_id:
            return
        self._tool_context_menu_callback(
            "results",
            tool_id,
            self._results_list.mapToGlobal(pos),
        )

    def _on_favorites_context_menu(self, pos) -> None:
        if self._tool_context_menu_callback is None:
            return
        tool_id = self.selected_favorite_tool_id()
        if not tool_id:
            return
        self._tool_context_menu_callback(
            "favorites",
            tool_id,
            self._favorites_list.mapToGlobal(pos),
        )

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
            star = "[FAV] " if tool_id in self._favorite_tool_ids else ""
            label = f"{star}{badge}{display_name} ({tool_id}) — {category}"

            self._results_list.addItem(QListWidgetItem(label))
            self._filtered_tool_ids.append(tool_id)
            matches += 1

        if matches == 0:
            hint = self._empty_state_message(
                query=query,
                show_available=show_available,
                show_locked=show_locked,
            )
            self._results_list.addItem(QListWidgetItem(hint))

        self._matches_label.setText(f"Matches: {matches}")

    def _empty_state_message(
        self,
        *,
        query: str,
        show_available: bool,
        show_locked: bool,
    ) -> str:
        if not show_available and not show_locked:
            return "No matches. Enable 'Show available' and/or 'Show locked'."
        if query:
            return "No matches. Try a broader search or adjust filters."
        return "No tools to display for the current filters."


def summarize_catalog(catalog: list[dict[str, Any]]) -> tuple[int, int]:
    available = 0
    locked = 0
    for item in catalog:
        if bool(item.get("locked", False)):
            locked += 1
        else:
            available += 1
    return available, locked
