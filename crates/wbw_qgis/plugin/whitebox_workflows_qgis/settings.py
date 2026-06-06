"""Whitebox Workflows QGIS Plugin – Settings Dialog (Phase 4).

Exposes practical runtime configuration, panel preferences, and backend version checking
without duplicating backend licensing authority.
"""
from __future__ import annotations

import threading
from typing import Any

try:
    from qgis.PyQt.QtCore import Qt, QTimer
    from qgis.PyQt.QtWidgets import (
        QCheckBox,
        QComboBox,
        QDialog,
        QDialogButtonBox,
        QFormLayout,
        QGroupBox,
        QHBoxLayout,
        QLabel,
        QLineEdit,
        QProgressBar,
        QPushButton,
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

        def setCurrentIndex(self, *_a, **_kw):
            return None

        def currentData(self):
            return "auto"

        def addItem(self, *_a, **_kw):
            return None

        def text(self):
            return ""

        def setText(self, *_a, **_kw):
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
    QHBoxLayout = _Dummy  # type: ignore[misc]
    QWidget = _Dummy  # type: ignore[misc]
    QLineEdit = _Dummy  # type: ignore[misc]
    QPushButton = _Dummy  # type: ignore[misc]
    QProgressBar = _Dummy  # type: ignore[misc]
    QTimer = _Dummy  # type: ignore[misc]
    Qt = object()  # type: ignore[assignment]


# ---------------------------------------------------------------------------
# Semantic style tokens (shared with panel styling, Phase 4).
# ---------------------------------------------------------------------------
STATUS_STYLES: dict[str, str] = {
    "ok": "color: #1B5E20; font-weight: bold;",
    "warning": "color: #E65100; font-weight: bold;",
    "bootstrap_error": "color: #B71C1C; font-weight: bold;",
    "error": "color: #B71C1C; font-weight: bold;",
    "unknown": "color: #616161;",
}

TIER_STYLES: dict[str, str] = {
    "pro": "color: #1A237E; font-weight: bold;",
    "enterprise": "color: #4A148C; font-weight: bold;",
    "open": "color: #1B5E20;",
    "unknown": "color: #616161;",
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
        panel_show_locked_recipes: bool = True,
        panel_width: int = 320,
        python_override_path: str = "",
        auto_install_backend: bool = True,
        auto_check_backend_updates: bool = True,
        installation_strategy: str = "",
    ):
        self.include_pro = bool(include_pro)
        self.tier = str(tier).strip() or "open"
        self.quick_open_top_match = bool(quick_open_top_match)
        self.panel_show_available = bool(panel_show_available)
        self.panel_show_locked = bool(panel_show_locked)
        self.panel_show_locked_recipes = bool(panel_show_locked_recipes)
        self.panel_width = max(220, min(520, int(panel_width)))
        # Empty string = use QGIS bundled Python; filled = use override path
        self.python_override_path = str(python_override_path).strip()
        self.auto_install_backend = bool(auto_install_backend)
        self.auto_check_backend_updates = bool(auto_check_backend_updates)
        # Track which installation strategy was last used (whiteboxgeo_wheel, pip_system_python, etc.)
        normalized_strategy = str(installation_strategy).strip().lower()
        self.installation_strategy = normalized_strategy if normalized_strategy in {"whiteboxgeo_wheel", "pip_system_python"} else ""

    def to_dict(self) -> dict[str, Any]:
        return {
            "include_pro": self.include_pro,
            "tier": self.tier,
            "quick_open_top_match": self.quick_open_top_match,
            "panel_show_available": self.panel_show_available,
            "panel_show_locked": self.panel_show_locked,
            "panel_show_locked_recipes": self.panel_show_locked_recipes,
            "panel_width": self.panel_width,
            "python_override_path": self.python_override_path,
            "auto_install_backend": self.auto_install_backend,
            "auto_check_backend_updates": self.auto_check_backend_updates,
            "installation_strategy": self.installation_strategy,
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
        self._update_in_progress = False
        self._current_version = ""
        self._available_version = ""

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

        # --- Runtime/Python group ---
        runtime_group = QGroupBox("Python & Backend")
        runtime_form = QFormLayout(runtime_group)

        # Include Pro and Tier
        self._include_pro_check = QCheckBox()
        self._include_pro_check.setChecked(s.include_pro)

        self._tier_combo = QComboBox()
        for t in ("open", "pro", "enterprise"):
            self._tier_combo.addItem(t)
        try:
            self._tier_combo.setCurrentText(s.tier)
        except Exception:
            pass

        # Python interpreter selection: dropdown + manual path
        self._python_combo = QComboBox()
        self._python_candidates: list[tuple[str, str]] = []
        self._python_manual_edit = QLineEdit()
        if hasattr(self._python_manual_edit, "setPlaceholderText"):
            self._python_manual_edit.setPlaceholderText("Leave empty for QGIS bundled Python")

        # Try to import bootstrap to discover Python candidates
        try:
            from . import bootstrap
            if hasattr(bootstrap, "discover_python_candidates"):
                self._python_candidates = bootstrap.discover_python_candidates()
        except Exception:
            pass

        # Populate dropdown with discovered candidates
        if self._python_candidates:
            self._python_combo.addItem("(select from discovered)", "")
            for path, label in self._python_candidates:
                display = f"{label}: {path}" if label else path
                self._python_combo.addItem(display, path)
            self._python_combo.addItem("(or enter path below)", "")
        else:
            self._python_combo.addItem("(enter path below)", "")

        # Set current value
        if s.python_override_path:
            if hasattr(self._python_manual_edit, "setText"):
                self._python_manual_edit.setText(s.python_override_path)

        # Python info display: path, version, install status
        self._python_path_label = QLabel()
        self._python_version_label = QLabel()
        self._python_status_label = QLabel()
        for label in [self._python_path_label, self._python_version_label, self._python_status_label]:
            if hasattr(label, "setWordWrap"):
                label.setWordWrap(True)
            if hasattr(label, "setStyleSheet"):
                label.setStyleSheet("color: #616161; font-size: 9pt;")

        # Auto-install backend
        self._auto_install_backend_check = QCheckBox()
        self._auto_install_backend_check.setChecked(s.auto_install_backend)

        # Auto-check updates
        self._auto_check_updates_check = QCheckBox()
        self._auto_check_updates_check.setChecked(s.auto_check_backend_updates)

        if hasattr(runtime_form, "addRow"):
            runtime_form.addRow("Include Pro catalog", self._include_pro_check)
            runtime_form.addRow("Requested tier", self._tier_combo)
            runtime_form.addRow("Python interpreter", self._python_combo)
            runtime_form.addRow("Manual path", self._python_manual_edit)
            runtime_form.addRow("Current path:", self._python_path_label)
            runtime_form.addRow("Detected version:", self._python_version_label)
            runtime_form.addRow("Backend status:", self._python_status_label)
            runtime_form.addRow("Auto-install backend when missing", self._auto_install_backend_check)
            runtime_form.addRow("Auto-check backend updates", self._auto_check_updates_check)

        # --- Panel group ---
        panel_group = QGroupBox("Panel Behaviour")
        panel_form = QFormLayout(panel_group)

        self._quick_open_check = QCheckBox()
        self._quick_open_check.setChecked(s.quick_open_top_match)

        self._show_available_check = QCheckBox()
        self._show_available_check.setChecked(s.panel_show_available)

        self._show_locked_check = QCheckBox()
        self._show_locked_check.setChecked(s.panel_show_locked)

        self._show_locked_recipes_check = QCheckBox()
        self._show_locked_recipes_check.setChecked(s.panel_show_locked_recipes)

        self._panel_width_spin = QSpinBox()
        if hasattr(self._panel_width_spin, "setRange"):
            self._panel_width_spin.setRange(220, 520)
        self._panel_width_spin.setValue(s.panel_width)

        if hasattr(panel_form, "addRow"):
            panel_form.addRow("Quick-open top match on Enter", self._quick_open_check)
            panel_form.addRow("Show available tools", self._show_available_check)
            panel_form.addRow("Show locked tools", self._show_locked_check)
            panel_form.addRow("Include locked recipes", self._show_locked_recipes_check)
            panel_form.addRow("Panel width (px)", self._panel_width_spin)

        # --- Backend Updates group ---
        updates_group = QGroupBox("Backend Updates")
        updates_form = QFormLayout(updates_group)
        
        self._current_version_label = QLabel()
        self._available_version_label = QLabel()
        self._update_status_label = QLabel()
        
        for label in [self._current_version_label, self._available_version_label, self._update_status_label]:
            if hasattr(label, "setWordWrap"):
                label.setWordWrap(True)
        
        self._update_button = QPushButton("Check for Updates")
        if hasattr(self._update_button, "clicked"):
            try:
                self._update_button.clicked.connect(self._on_check_updates)
            except Exception:
                pass
        
        self._update_progress = QProgressBar()
        self._update_progress.setVisible(False)
        
        if hasattr(updates_form, "addRow"):
            updates_form.addRow("Current version:", self._current_version_label)
            updates_form.addRow("Available version:", self._available_version_label)
            updates_form.addRow("Status:", self._update_status_label)
            updates_form.addRow(self._update_button)
            updates_form.addRow(self._update_progress)

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
            outer.addWidget(updates_group)
            outer.addWidget(self._entitlement_label)
            if buttons is not None:
                outer.addWidget(buttons)

        if hasattr(self, "setLayout"):
            self.setLayout(outer)
        
        # Fetch version info in background after dialog is shown
        self._schedule_version_fetch()

    # ------------------------------------------------------------------
    # Version checking and update management
    # ------------------------------------------------------------------
    def _schedule_version_fetch(self) -> None:
        """Schedule version info fetching in background thread."""
        try:
            timer = QTimer()
            timer.singleShot(100, self._start_version_fetch_thread)  # type: ignore[attr-defined]
        except Exception:
            # Qt not available or timer failed; try direct fetch
            self._fetch_versions_thread()

    def _start_version_fetch_thread(self) -> None:
        """Start background thread to fetch versions."""
        thread = threading.Thread(target=self._fetch_versions_thread, daemon=True)
        thread.start()

    def _fetch_versions_thread(self) -> None:
        """Fetch current and available versions in background.
        
        NOTE: Reads python_path in the calling thread (safe) rather than from
        Qt widget in background thread (not thread-safe).
        """
        try:
            from . import bootstrap
            import sys
            
            # Read python_path in this thread (before moving to background thread)
            # This ensures we read the current state, not a stale reference
            try:
                python_path = str(self._python_manual_edit.text()).strip()
            except Exception:
                python_path = ""
            
            # Get installed version
            if python_path:
                current = bootstrap.get_whitebox_workflows_version_for_interpreter(python_path)
            else:
                # Use current Python (QGIS bundled)
                current = bootstrap.get_whitebox_workflows_version_for_interpreter(sys.executable)
            
            # Get available version
            available = bootstrap.get_latest_wheel_version()
            
            # Update state
            self._current_version = current
            self._available_version = available
            
            # Update UI from main thread
            try:
                timer = QTimer()
                timer.singleShot(0, self._update_version_display)  # type: ignore[attr-defined]
            except Exception:
                self._update_version_display()
        except Exception:
            pass

    def _update_version_display(self) -> None:
        """Update version display labels with fetched info."""
        try:
            current = self._current_version or "(not installed)"
            available = self._available_version or "(check failed)"
            
            if hasattr(self._current_version_label, "setText"):
                self._current_version_label.setText(current)
            if hasattr(self._available_version_label, "setText"):
                self._available_version_label.setText(available)
            
            # Determine status using semantic version comparison
            # NOTE: Must use semantic comparison, not string comparison
            # (e.g., "2.10.0" > "2.9.0" would fail with lexical comparison)
            from . import bootstrap
            update_available = (
                self._available_version and self._current_version and
                bootstrap._compare_versions_semantic(self._available_version, self._current_version)
            )
            
            if not self._current_version:
                status_text = "Not installed – click Update to install"
                status_color = "color: #E65100; font-weight: bold;"  # warning orange
            elif not self._available_version:
                status_text = "Unable to check for updates"
                status_color = "color: #616161;"  # neutral gray
            elif update_available:
                status_text = f"Update available ({self._available_version})"
                status_color = "color: #1565C0; font-weight: bold;"  # info blue
                # Update button text to "Update Available"
                if hasattr(self._update_button, "setText"):
                    self._update_button.setText("Update to " + self._available_version)
            else:
                status_text = "Up to date"
                status_color = "color: #1B5E20; font-weight: bold;"  # ok green
                # Disable update button
                if hasattr(self._update_button, "setEnabled"):
                    self._update_button.setEnabled(False)
            
            if hasattr(self._update_status_label, "setText"):
                self._update_status_label.setText(status_text)
            if hasattr(self._update_status_label, "setStyleSheet"):
                self._update_status_label.setStyleSheet(status_color)
        except Exception:
            pass

    def _on_check_updates(self) -> None:
        """Handle Check/Update button click."""
        if self._update_in_progress:
            return
        
        self._update_in_progress = True
        
        try:
            if hasattr(self._update_button, "setEnabled"):
                self._update_button.setEnabled(False)
            if hasattr(self._update_progress, "setVisible"):
                self._update_progress.setVisible(True)
            if hasattr(self._update_status_label, "setText"):
                self._update_status_label.setText("Downloading and installing...")
        except Exception:
            pass
        
        # Start update in background thread
        thread = threading.Thread(target=self._perform_update_thread, daemon=True)
        thread.start()

    def _perform_update_thread(self) -> None:
        """Download and install wheels in background.
        
        NOTE: Captures result/error before returning to main thread to avoid
        closure issues with mutable state.
        """
        result_dict = None
        error_msg = None
        
        try:
            from . import bootstrap
            
            # Get version to download
            version = self._available_version or ""
            
            # Download and install
            result_dict = bootstrap.download_and_install_wheels(version)
        except Exception as exc:
            error_msg = str(exc)
        
        # Update display from main thread with captured state
        if error_msg:
            try:
                timer = QTimer()
                # Use functools.partial or explicit wrapper to avoid closure issues
                timer.singleShot(0, lambda err=error_msg: self._update_install_error(err))  # type: ignore[attr-defined]
            except Exception:
                self._update_install_error(error_msg)
        elif result_dict:
            try:
                timer = QTimer()
                # Capture result_dict in default argument to avoid closure issues
                timer.singleShot(0, lambda res=result_dict: self._update_install_result(res))  # type: ignore[attr-defined]
            except Exception:
                self._update_install_result(result_dict)

    def _update_install_result(self, result: dict) -> None:
        """Update UI after installation attempt."""
        self._update_in_progress = False
        try:
            success = result.get("success", False)
            message = result.get("message", "")
            
            if hasattr(self._update_progress, "setVisible"):
                self._update_progress.setVisible(False)
            
            if success:
                # Refresh version display
                self._current_version = result.get("version", "")
                self._update_version_display()
                if hasattr(self._update_status_label, "setText"):
                    self._update_status_label.setText("✓ Installation successful – restart QGIS to load new version")
                if hasattr(self._update_status_label, "setStyleSheet"):
                    self._update_status_label.setStyleSheet("color: #1B5E20; font-weight: bold;")
                if hasattr(self._update_button, "setText"):
                    self._update_button.setText("Check for Updates")
                if hasattr(self._update_button, "setEnabled"):
                    self._update_button.setEnabled(False)
            else:
                if hasattr(self._update_status_label, "setText"):
                    self._update_status_label.setText(f"✗ Installation failed: {message}")
                if hasattr(self._update_status_label, "setStyleSheet"):
                    self._update_status_label.setStyleSheet("color: #B71C1C; font-weight: bold;")
                if hasattr(self._update_button, "setEnabled"):
                    self._update_button.setEnabled(True)
        except Exception:
            pass

    def _update_install_error(self, error_msg: str) -> None:
        """Update UI after installation error."""
        self._update_in_progress = False
        try:
            if hasattr(self._update_progress, "setVisible"):
                self._update_progress.setVisible(False)
            if hasattr(self._update_status_label, "setText"):
                self._update_status_label.setText(f"✗ Error: {error_msg[:60]}")
            if hasattr(self._update_status_label, "setStyleSheet"):
                self._update_status_label.setStyleSheet("color: #B71C1C; font-weight: bold;")
            if hasattr(self._update_button, "setEnabled"):
                self._update_button.setEnabled(True)
        except Exception:
            pass

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
        # Get Python override path from manual edit field
        python_path = str(self._python_manual_edit.text()).strip()
        
        return WhiteboxPluginSettings(
            include_pro=bool(self._include_pro_check.isChecked()),
            tier=str(self._tier_combo.currentText()).strip() or "open",
            quick_open_top_match=bool(self._quick_open_check.isChecked()),
            panel_show_available=bool(self._show_available_check.isChecked()),
            panel_show_locked=bool(self._show_locked_check.isChecked()),
            panel_show_locked_recipes=bool(self._show_locked_recipes_check.isChecked()),
            panel_width=int(self._panel_width_spin.value()),
            python_override_path=python_path,
            auto_install_backend=bool(self._auto_install_backend_check.isChecked()),
            auto_check_backend_updates=bool(self._auto_check_updates_check.isChecked()),
        )
