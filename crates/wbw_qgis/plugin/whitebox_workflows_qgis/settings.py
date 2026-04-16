"""Whitebox Workflows QGIS Plugin – Settings Dialog (Phase 3).

Exposes practical runtime configuration and panel preferences without
duplicating backend licensing authority.
"""
from __future__ import annotations

from typing import Any

try:
    from qgis.PyQt.QtCore import Qt
    from qgis.PyQt.QtWidgets import (
        QCheckBox,
        QComboBox,
        QDialog,
        QDialogButtonBox,
        QFormLayout,
        QGroupBox,
        QLabel,
        QSpinBox,
        QVBoxLayout,
        QWidget,
    )
    _HAS_QT = True
except Exception:  # pragma: no cover
    _HAS_QT = False

    class _Dummy:  # type: ignore[override]
        def __init__(self, *_a, **_kw):
            pass

        def setChecked(self, *_a, **_kw):
            return None

        def isChecked(self):
            return True

        def value(self):
            return 320

        def setValue(self, *_a, **_kw):
            return None

        def currentText(self):
            return "open"

        def setCurrentText(self, *_a, **_kw):
            return None

        def addItem(self, *_a, **_kw):
            return None

        def exec(self):
            return 0

        def exec_(self):
            return 0

        def accepted(self):
            return None

    QDialog = _Dummy  # type: ignore[misc]
    QDialogButtonBox = _Dummy  # type: ignore[misc]
    QFormLayout = _Dummy  # type: ignore[misc]
    QGroupBox = _Dummy  # type: ignore[misc]
    QLabel = _Dummy  # type: ignore[misc]
    QCheckBox = _Dummy  # type: ignore[misc]
    QComboBox = _Dummy  # type: ignore[misc]
    QSpinBox = _Dummy  # type: ignore[misc]
    QVBoxLayout = _Dummy  # type: ignore[misc]
    QWidget = _Dummy  # type: ignore[misc]
    Qt = object()  # type: ignore[assignment]


# ---------------------------------------------------------------------------
# Semantic style tokens (shared with panel styling, Phase 4).
# ---------------------------------------------------------------------------
STATUS_STYLES: dict[str, str] = {
    "ok":              "color: #1B5E20; font-weight: bold;",
    "warning":         "color: #E65100; font-weight: bold;",
    "bootstrap_error": "color: #B71C1C; font-weight: bold;",
    "error":           "color: #B71C1C; font-weight: bold;",
    "unknown":         "color: #616161;",
}

TIER_STYLES: dict[str, str] = {
    "pro":        "color: #1A237E; font-weight: bold;",
    "enterprise": "color: #4A148C; font-weight: bold;",
    "open":       "color: #1B5E20;",
    "unknown":    "color: #616161;",
}

LOCKED_LABEL_STYLE = "color: #F57F17;"   # amber – tool visible but locked
AVAILABLE_LABEL_STYLE = "color: #1B5E20;"  # green – tool available


def status_style(status: str) -> str:
    """Return the CSS style string for a given status token."""
    return STATUS_STYLES.get(str(status).strip().lower(), STATUS_STYLES["unknown"])


def tier_style(tier: str) -> str:
    """Return the CSS style string for a given tier string."""
    key = str(tier).strip().lower().split("_")[0]  # "pro_legacy" → "pro"
    return TIER_STYLES.get(key, TIER_STYLES["unknown"])


# ---------------------------------------------------------------------------
# Settings snapshot (pure data, no Qt dependency).
# ---------------------------------------------------------------------------
class WhiteboxPluginSettings:
    """Holds the current plugin preference state."""

    def __init__(
        self,
        *,
        include_pro: bool = True,
        tier: str = "open",
        quick_open_top_match: bool = True,
        panel_show_available: bool = True,
        panel_show_locked: bool = True,
        panel_width: int = 320,
    ):
        self.include_pro = bool(include_pro)
        self.tier = str(tier).strip() or "open"
        self.quick_open_top_match = bool(quick_open_top_match)
        self.panel_show_available = bool(panel_show_available)
        self.panel_show_locked = bool(panel_show_locked)
        self.panel_width = max(220, min(520, int(panel_width)))

    def to_dict(self) -> dict[str, Any]:
        return {
            "include_pro": self.include_pro,
            "tier": self.tier,
            "quick_open_top_match": self.quick_open_top_match,
            "panel_show_available": self.panel_show_available,
            "panel_show_locked": self.panel_show_locked,
            "panel_width": self.panel_width,
        }


# ---------------------------------------------------------------------------
# Settings dialog.
# ---------------------------------------------------------------------------
class WhiteboxSettingsDialog(QDialog):
    """Modal dialog for editing Whitebox Workflows plugin preferences."""

    def __init__(self, settings: WhiteboxPluginSettings, parent=None):
        super().__init__(parent)
        self._settings = settings
        self._accepted = False

        try:
            if hasattr(self, "setWindowTitle"):
                self.setWindowTitle("Whitebox Workflows – Plugin Settings")
            if hasattr(self, "setMinimumWidth"):
                self.setMinimumWidth(400)
        except Exception:
            pass

        self._build_ui(settings)

    # ------------------------------------------------------------------
    def _build_ui(self, s: WhiteboxPluginSettings) -> None:
        outer = QVBoxLayout(self)

        # --- Runtime group ---
        runtime_group = QGroupBox("Runtime Discovery")
        runtime_form = QFormLayout(runtime_group)

        self._include_pro_check = QCheckBox()
        self._include_pro_check.setChecked(s.include_pro)

        self._tier_combo = QComboBox()
        for t in ("open", "pro", "enterprise"):
            self._tier_combo.addItem(t)
        try:
            self._tier_combo.setCurrentText(s.tier)
        except Exception:
            pass

        if hasattr(runtime_form, "addRow"):
            runtime_form.addRow("Include Pro catalog", self._include_pro_check)
            runtime_form.addRow("Requested tier", self._tier_combo)

        # --- Panel group ---
        panel_group = QGroupBox("Panel Behaviour")
        panel_form = QFormLayout(panel_group)

        self._quick_open_check = QCheckBox()
        self._quick_open_check.setChecked(s.quick_open_top_match)

        self._show_available_check = QCheckBox()
        self._show_available_check.setChecked(s.panel_show_available)

        self._show_locked_check = QCheckBox()
        self._show_locked_check.setChecked(s.panel_show_locked)

        self._panel_width_spin = QSpinBox()
        if hasattr(self._panel_width_spin, "setRange"):
            self._panel_width_spin.setRange(220, 520)
        self._panel_width_spin.setValue(s.panel_width)

        if hasattr(panel_form, "addRow"):
            panel_form.addRow("Quick-open top match on Enter", self._quick_open_check)
            panel_form.addRow("Show available tools", self._show_available_check)
            panel_form.addRow("Show locked tools", self._show_locked_check)
            panel_form.addRow("Panel width (px)", self._panel_width_spin)

        # --- Entitlement notice ---
        self._entitlement_label = QLabel(
            "Changes to Include Pro / Tier take effect after the next Catalog Refresh."
        )
        if hasattr(self._entitlement_label, "setWordWrap"):
            self._entitlement_label.setWordWrap(True)
        if hasattr(self._entitlement_label, "setStyleSheet"):
            self._entitlement_label.setStyleSheet("color: #616161; font-style: italic;")

        # --- Standard Ok/Cancel buttons ---
        try:
            buttons = QDialogButtonBox(
                QDialogButtonBox.StandardButton.Ok | QDialogButtonBox.StandardButton.Cancel
            )
        except Exception:
            try:
                buttons = QDialogButtonBox(
                    QDialogButtonBox.Ok | QDialogButtonBox.Cancel
                )
            except Exception:
                buttons = None  # type: ignore[assignment]

        if buttons is not None:
            accepted_signal = getattr(buttons, "accepted", None)
            rejected_signal = getattr(buttons, "rejected", None)
            if accepted_signal is not None:
                accepted_signal.connect(self._on_accept)
            if rejected_signal is not None:
                rejected_signal.connect(self._on_reject)

        if hasattr(outer, "addWidget"):
            outer.addWidget(runtime_group)
            outer.addWidget(panel_group)
            outer.addWidget(self._entitlement_label)
            if buttons is not None:
                outer.addWidget(buttons)

        if hasattr(self, "setLayout"):
            self.setLayout(outer)

    # ------------------------------------------------------------------
    def _on_accept(self):
        self._accepted = True
        close = getattr(self, "accept", None)
        if callable(close):
            close()

    def _on_reject(self):
        close = getattr(self, "reject", None)
        if callable(close):
            close()

    def was_accepted(self) -> bool:
        return self._accepted

    def read_settings(self) -> WhiteboxPluginSettings:
        """Return a settings snapshot populated from the current dialog state."""
        return WhiteboxPluginSettings(
            include_pro=bool(self._include_pro_check.isChecked()),
            tier=str(self._tier_combo.currentText()).strip() or "open",
            quick_open_top_match=bool(self._quick_open_check.isChecked()),
            panel_show_available=bool(self._show_available_check.isChecked()),
            panel_show_locked=bool(self._show_locked_check.isChecked()),
            panel_width=int(self._panel_width_spin.value()),
        )
