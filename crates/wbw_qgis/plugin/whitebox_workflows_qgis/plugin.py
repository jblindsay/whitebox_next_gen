from __future__ import annotations

import json
import os
from datetime import datetime

from .bootstrap import (
    backend_install_status,
    backend_update_status,
    install_or_upgrade_whitebox_workflows,
    is_backend_not_installed_error,
    set_runtime_preferences,
)
from .diagnostics import diagnostics_text, gather_runtime_diagnostics
from .host_api import (
    host_capabilities,
    open_local_file,
    open_processing_algorithm_dialog,
    push_host_message,
    qgis_version_string,
    register_dock_widget,
    register_plugin_action,
    run_dialog,
    run_menu,
    register_provider,
    unregister_dock_widget,
    unregister_plugin_action,
    unregister_provider,
    show_info_dialog,
)
from .panel import WhiteboxDockPanel, summarize_catalog
from .provider import WhiteboxProcessingProvider
from .recipes import (
    ensure_user_recipe_file,
    load_all_recipe_definitions,
    recipe_steps_text,
    user_recipe_file_path,
    visible_recipes,
)
from .settings import WhiteboxPluginSettings, WhiteboxSettingsDialog

try:
    from qgis.PyQt.QtGui import QAction, QIcon
    from qgis.PyQt.QtCore import QSettings
    from qgis.PyQt.QtWidgets import QApplication, QInputDialog, QLineEdit, QMenu, QMessageBox
except Exception:  # pragma: no cover
    class QAction:  # type: ignore[override]
        def __init__(self, *_args, **_kwargs):
            self.triggered = _Signal()

        def setIcon(self, *_args, **_kwargs):
            return None

    class QIcon:  # type: ignore[override]
        def __init__(self, *_args, **_kwargs):
            pass

    class QApplication:  # type: ignore[override]
        @staticmethod
        def clipboard():
            class _Clipboard:
                def setText(self, *_args, **_kwargs):
                    return None

            return _Clipboard()

    class QMenu:  # type: ignore[override]
        def __init__(self, *_args, **_kwargs):
            pass

        class _Action:
            def setEnabled(self, *_args, **_kwargs):
                return None

        def addAction(self, _label):
            return self._Action()

        def exec(self, *_args, **_kwargs):
            return None

        def exec_(self, *_args, **_kwargs):
            return None

    class QInputDialog:  # type: ignore[override]
        @staticmethod
        def getText(*_args, **_kwargs):
            return "", False

    class QMessageBox:  # type: ignore[override]
        Yes = 1
        No = 0

        class StandardButton:
            Yes = 1
            No = 0

        class ButtonRole:
            AcceptRole = 0
            RejectRole = 1
            DestructiveRole = 2

        @staticmethod
        def question(*_args, **_kwargs):
            return QMessageBox.No

        def __init__(self, *_args, **_kwargs):
            self._clicked = None

        def setWindowTitle(self, *_args, **_kwargs):
            return None

        def setText(self, *_args, **_kwargs):
            return None

        def addButton(self, *_args, **_kwargs):
            button = object()
            if self._clicked is None:
                self._clicked = button
            return button

        def setDefaultButton(self, *_args, **_kwargs):
            return None

        def clickedButton(self):
            return self._clicked

    class QLineEdit:  # type: ignore[override]
        class EchoMode:
            Normal = 0

    class QSettings:  # type: ignore[override]
        def value(self, *_args, **_kwargs):
            return ""

        def setValue(self, *_args, **_kwargs):
            return None

    class _Signal:  # type: ignore[override]
        def connect(self, *_args, **_kwargs):
            return None


class WhiteboxWorkflowsPlugin:
    def __init__(self, iface):
        self.iface = iface
        self.provider = WhiteboxProcessingProvider(iface=iface)
        self._provider_registered = False
        self._menu_label = "&Whitebox Workflows"
        self._diagnostics_action = None
        self._refresh_action = None
        self._panel_action = None
        self._settings_action = None
        self._activate_license_action = None
        self._deactivate_license_action = None
        self._transfer_license_action = None
        self._backend_updates_action = None
        self._dock_panel = None
        self._recipe_index: dict[str, dict] = {}
        self._recent_tool_ids: list[str] = []
        self._favorite_tool_ids: list[str] = []
        self._favorite_defaults_applied = False
        self._has_persisted_favorites = False
        self._max_recent_tools = 8
        self._settings_key_recent = "whitebox_workflows/recent_tools"
        self._settings_key_favorites = "whitebox_workflows/favorite_tools"
        self._settings_key_quick_open = "whitebox_workflows/quick_open_top_match"
        self._settings_key_include_pro = "whitebox_workflows/include_pro"
        self._settings_key_requested_tier = "whitebox_workflows/requested_tier"
        self._quick_open_top_match = True
        self._settings_key_panel_visible = "whitebox_workflows/panel_visible"
        self._settings_key_panel_width = "whitebox_workflows/panel_width"
        self._settings_key_show_available = "whitebox_workflows/show_available"
        self._settings_key_show_locked = "whitebox_workflows/show_locked"
        self._settings_key_show_locked_recipes = "whitebox_workflows/show_locked_recipes"
        self._settings_key_search_text = "whitebox_workflows/search_text"
        self._settings_key_focus_area = "whitebox_workflows/focus_area"
        self._settings_key_last_tool = "whitebox_workflows/last_tool_id"
        self._settings_key_runtime_mode = "whitebox_workflows/runtime_mode"
        self._settings_key_local_python = "whitebox_workflows/local_python"
        self._settings_key_auto_install_backend = "whitebox_workflows/auto_install_backend"
        self._settings_key_auto_check_updates = "whitebox_workflows/auto_check_updates"
        self._settings_key_skip_auto_update_checks_local_mode = "whitebox_workflows/skip_auto_update_checks_local_mode"
        self._settings_key_last_update_check_unix = "whitebox_workflows/last_update_check_unix"
        self._settings_key_skipped_update_version = "whitebox_workflows/skipped_update_version"
        self._panel_visible = True
        self._panel_width = 320
        self._panel_show_available = True
        self._panel_show_locked = True
        self._panel_show_locked_recipes = True
        self._panel_search_text = ""
        self._panel_focus_area = "search"
        self._last_tool_id = ""
        self._last_runtime_warning_signature = ""
        self._runtime_mode = "auto"
        self._runtime_local_python = ""
        self._auto_install_backend = True
        self._auto_check_backend_updates = True
        self._skip_auto_update_checks_in_local_mode = True
        self._last_update_check_unix = 0
        self._skipped_update_version = ""

    def _plugin_icon(self):
        base_dir = os.path.dirname(__file__)
        candidates = (
            os.path.join(base_dir, "icons", "WbW.png"),
            os.path.join(base_dir, "icons", "WbW.svg"),
        )
        for path in candidates:
            if os.path.exists(path):
                return QIcon(path)
        return QIcon()

    def _plugin_icon_named(self, icon_name: str):
        base_dir = os.path.dirname(__file__)
        candidates = (
            os.path.join(base_dir, "icons", f"{icon_name}.png"),
            os.path.join(base_dir, "icons", f"{icon_name}.svg"),
            os.path.join(base_dir, "icons", "WbW.png"),
            os.path.join(base_dir, "icons", "WbW.svg"),
        )
        for path in candidates:
            if os.path.exists(path):
                return QIcon(path)
        return QIcon()

    def _panel_action_icon(self):
        return self._plugin_icon_named("WbW_panel")

    def _refresh_action_icon(self):
        return self._plugin_icon_named("WbW_refresh")

    def _diagnostics_action_icon(self):
        return self._plugin_icon_named("WbW_diagnostics")

    def _settings_action_icon(self):
        return self._plugin_icon_named("WbW_settings")

    def _license_action_icon(self):
        return self._plugin_icon_named("WbW_settings")

    def _backend_updates_action_icon(self):
        return self._plugin_icon_named("WbW_refresh")

    def initGui(self):
        # Support QGIS 3/4 from one codebase; unsupported majors are rejected.
        host_info = host_capabilities(self.iface)
        major = int(host_info.get("major", 0) or 0)
        if not bool(host_info.get("supported_major", False)):
            push_host_message(
                self.iface,
                "Whitebox Workflows",
                f"Unsupported QGIS major version: {major or 'unknown'}.",
                level="warning",
            )
            return

        if host_info.get("partial"):
            missing = ", ".join(host_info.get("missing_required") or [])
            push_host_message(
                self.iface,
                "Whitebox Workflows",
                f"Host compatibility is partial; some features may be unavailable ({missing}).",
                level="warning",
            )

        self._load_recent_tools()
        self._load_favorite_tools()
        self._load_runtime_preferences()
        self._load_backend_preferences()
        self._apply_runtime_preferences_to_bootstrap()
        self._load_quick_open_preference()
        self._load_panel_ui_state()
        self._load_last_tool()

        self._install_panel()
        self._install_actions()

        if not self._ensure_backend_available(interactive=False):
            self._ensure_backend_available(interactive=True)

        # On first run, derive runtime tier defaults from discovered
        # capabilities instead of hard-coding Pro defaults.
        self._initialize_entitlement_runtime_defaults()

        # Ensure backend install/activation checks run before provider
        # registration, because addProvider() may eagerly call provider.load().
        if not register_provider(self.iface, self.provider):
            return
        self._provider_registered = True

        self._refresh_catalog(silent=True)
        self._check_backend_updates(manual=False)

        # Helpful startup message where the host exposes a message bar.
        msg = f"Whitebox Workflows provider loaded (QGIS {qgis_version_string() or 'unknown'})."
        if host_info.get("partial"):
            msg += " Compatibility mode enabled."
        push_host_message(self.iface, "Whitebox Workflows", msg, level="info")

    def _install_actions(self):
        panel_action = QAction("Show Whitebox Panel", self.iface.mainWindow())
        if hasattr(panel_action, "setIcon"):
            panel_action.setIcon(self._panel_action_icon())
        panel_action.triggered.connect(self._toggle_panel)
        if register_plugin_action(self.iface, panel_action, self._menu_label):
            self._panel_action = panel_action

        refresh_action = QAction("Refresh Catalog + Help", self.iface.mainWindow())
        if hasattr(refresh_action, "setIcon"):
            refresh_action.setIcon(self._refresh_action_icon())
        refresh_action.triggered.connect(self._refresh_catalog)
        if register_plugin_action(self.iface, refresh_action, self._menu_label):
            self._refresh_action = refresh_action

        diagnostics_action = QAction("Runtime Diagnostics", self.iface.mainWindow())
        if hasattr(diagnostics_action, "setIcon"):
            diagnostics_action.setIcon(self._diagnostics_action_icon())
        diagnostics_action.triggered.connect(self._show_diagnostics)
        if register_plugin_action(self.iface, diagnostics_action, self._menu_label):
            self._diagnostics_action = diagnostics_action

        settings_action = QAction("Plugin Settings", self.iface.mainWindow())
        if hasattr(settings_action, "setIcon"):
            settings_action.setIcon(self._settings_action_icon())
        settings_action.triggered.connect(self._show_settings)
        if register_plugin_action(self.iface, settings_action, self._menu_label):
            self._settings_action = settings_action

        activate_license_action = QAction("Activate License", self.iface.mainWindow())
        if hasattr(activate_license_action, "setIcon"):
            activate_license_action.setIcon(self._license_action_icon())
        activate_license_action.triggered.connect(self._activate_license)
        if register_plugin_action(self.iface, activate_license_action, self._menu_label):
            self._activate_license_action = activate_license_action

        deactivate_license_action = QAction("Deactivate License", self.iface.mainWindow())
        if hasattr(deactivate_license_action, "setIcon"):
            deactivate_license_action.setIcon(self._license_action_icon())
        deactivate_license_action.triggered.connect(self._deactivate_license)
        if register_plugin_action(self.iface, deactivate_license_action, self._menu_label):
            self._deactivate_license_action = deactivate_license_action

        transfer_license_action = QAction("Transfer License", self.iface.mainWindow())
        if hasattr(transfer_license_action, "setIcon"):
            transfer_license_action.setIcon(self._license_action_icon())
        transfer_license_action.triggered.connect(self._transfer_license)
        if register_plugin_action(self.iface, transfer_license_action, self._menu_label):
            self._transfer_license_action = transfer_license_action

        backend_updates_action = QAction("Check Backend Updates", self.iface.mainWindow())
        if hasattr(backend_updates_action, "setIcon"):
            backend_updates_action.setIcon(self._backend_updates_action_icon())
        backend_updates_action.triggered.connect(self._manual_check_backend_updates)
        if register_plugin_action(self.iface, backend_updates_action, self._menu_label):
            self._backend_updates_action = backend_updates_action

    def _prompt_text(self, title: str, label: str, default: str = "", password: bool = False):
        try:
            mode = QLineEdit.EchoMode.Normal
            if password and hasattr(QLineEdit.EchoMode, "Password"):
                mode = QLineEdit.EchoMode.Password
            value, ok = QInputDialog.getText(self.iface.mainWindow(), title, label, mode, default)
            return str(value).strip(), bool(ok)
        except Exception:
            return "", False

    def _activate_license(self, *_args):
        from .bootstrap import invoke_license_function

        key, ok = self._prompt_text("Activate Whitebox License", "License key")
        if not ok or not key:
            return
        firstname, ok = self._prompt_text("Activate Whitebox License", "First name")
        if not ok or not firstname:
            return
        lastname, ok = self._prompt_text("Activate Whitebox License", "Last name")
        if not ok or not lastname:
            return
        email, ok = self._prompt_text("Activate Whitebox License", "Email")
        if not ok or not email:
            return

        # Use production server by default; developers can override via WBW_LICENSE_PROVIDER_URL env var
        provider_url = os.environ.get(
            "WBW_LICENSE_PROVIDER_URL", "https://radiant-garden-01227.herokuapp.com"
        )

        try:
            message = invoke_license_function(
                "activate_license",
                key=key,
                firstname=firstname,
                lastname=lastname,
                email=email,
                provider_url=provider_url,
            )
            self._notify_info(str(message))
            self._refresh_catalog(silent=True)
        except Exception as exc:
            self._notify_warning(f"License activation failed: {exc}")

    def _deactivate_license(self, *_args):
        from .bootstrap import invoke_license_function

        try:
            message = invoke_license_function("deactivate_license", from_transfer=False)
            self._notify_info(str(message))
            self._refresh_catalog(silent=True)
        except Exception as exc:
            self._notify_warning(f"License deactivation failed: {exc}")

    def _transfer_license(self, *_args):
        from .bootstrap import invoke_license_function

        try:
            payload_raw = invoke_license_function("transfer_license")
            payload = payload_raw
            if isinstance(payload_raw, str):
                try:
                    payload = json.loads(payload_raw)
                except Exception:
                    payload = {"message": payload_raw}

            message = str(payload.get("message", "License transferred.")) if isinstance(payload, dict) else str(payload)
            if isinstance(payload, dict):
                try:
                    QApplication.clipboard().setText(json.dumps(payload, indent=2))
                except Exception:
                    pass
            self._notify_info(message)
            self._refresh_catalog(silent=True)
        except Exception as exc:
            self._notify_warning(f"License transfer failed: {exc}")

    def _install_panel(self):
        panel = WhiteboxDockPanel(self.iface.mainWindow())
        if hasattr(panel, "setWindowIcon"):
            panel.setWindowIcon(self._plugin_icon())
        panel.on_refresh(self._refresh_catalog)
        panel.on_diagnostics(self._show_diagnostics)
        panel.on_open_tool(self._open_tool_from_panel)
        panel.on_open_recent_tool(self._open_tool_from_recent)
        panel.on_open_favorite_tool(self._open_tool_from_favorite)
        panel.on_open_recipe(self._open_recipe_from_panel)
        panel.on_copy_recipe_steps(self._copy_recipe_steps_from_panel)
        panel.on_show_recipe_upgrade(self._show_recipe_upgrade_from_panel)
        panel.on_open_recipe_file(self._open_recipe_file_from_panel)
        panel.on_reload_recipe_file(self._reload_recipe_file_from_panel)
        panel.on_validate_recipe_file(self._validate_recipe_file_from_panel)
        panel.on_add_favorite(self._add_selected_favorite)
        panel.on_remove_favorite(self._remove_selected_favorite)
        panel.on_toggle_selected_favorite(self._toggle_selected_result_favorite)
        panel.on_remove_selected_favorite_shortcut(self._remove_selected_favorite)
        panel.on_move_favorite_up(self._move_selected_favorite_up)
        panel.on_move_favorite_down(self._move_selected_favorite_down)
        panel.on_clear_favorites(self._clear_favorites)
        panel.on_clear_recents(self._clear_recents)
        panel.on_quick_open_toggled(self._on_quick_open_toggled)
        panel.on_filter_state_changed(self._on_filter_state_changed)
        panel.on_search_state_changed(self._on_search_state_changed)
        panel.on_recipe_discovery_state_changed(self._on_recipe_discovery_state_changed)
        panel.on_focus_area_changed(self._on_focus_area_changed)
        panel.on_session_banner_clicked(self._show_diagnostics)
        panel.on_tool_context_menu(self._show_tool_context_menu)
        if register_dock_widget(self.iface, panel):
            self._dock_panel = panel
            panel.set_quick_open_enabled(self._quick_open_top_match)
            panel.set_show_available_enabled(self._panel_show_available)
            panel.set_show_locked_enabled(self._panel_show_locked)
            panel.set_show_locked_recipes_enabled(self._panel_show_locked_recipes)
            panel.set_search_text(self._panel_search_text)
            panel.set_focus_area(self._panel_focus_area)
            resize = getattr(panel, "resize", None)
            if callable(resize):
                height = getattr(panel, "height", None)
                h = int(height()) if callable(height) else 600
                resize(int(self._panel_width), h)
            set_visible = getattr(panel, "setVisible", None)
            if callable(set_visible):
                set_visible(True)
                self._panel_visible = True

    def _open_tool_from_panel(self, tool_id: str):
        tool_id_lc = str(tool_id).strip().lower()
        if tool_id_lc == "field_calculator":
            self._open_field_calculator_assistant(tool_id)
            return
        if tool_id_lc == "raster_calculator":
            self._open_raster_calculator_assistant(tool_id)
            return

        provider_id = self.provider.id()
        opened = open_processing_algorithm_dialog(self.iface, provider_id, tool_id)
        if opened:
            self._record_last_tool(tool_id)
            self._record_recent_tool(tool_id)
            self._notify_info(f"Opening tool: {tool_id}")
        else:
            self._notify_warning(
                f"Unable to open dialog for {tool_id}; host processing API not available."
            )

    def _open_field_calculator_assistant(self, tool_id: str):
        self._notify_info(
            "Opening Field Calculator Assistant. After review, the standard processing dialog will open with prefilled parameters."
        )
        try:
            from .field_calculator_dialog import run_field_calculator_assistant

            launch_params = run_field_calculator_assistant(
                self.iface,
                include_pro=self.provider.include_pro,
                tier=self.provider.tier,
            )
        except Exception as exc:
            self._notify_warning(
                f"Field Calculator assistant unavailable; panel launch requires assistant ({exc})."
            )
            return

        if launch_params == {}:
            # Assistant was opened and then cancelled.
            return

        provider_id = self.provider.id()
        opened = open_processing_algorithm_dialog(
            self.iface,
            provider_id,
            tool_id,
            params=launch_params,
        )
        if opened:
            self._record_last_tool(tool_id)
            self._record_recent_tool(tool_id)
            if launch_params:
                self._notify_info(
                    "Opened Field Calculator processing dialog with assistant parameters."
                )
            else:
                self._notify_info("Opening tool: field_calculator")
        else:
            self._notify_warning(
                "Unable to open dialog for field_calculator; host processing API not available."
            )

    def _open_raster_calculator_assistant(self, tool_id: str):
        self._notify_info(
            "Opening Raster Calculator Assistant. After review, the standard processing dialog will open with prefilled parameters."
        )
        try:
            from .raster_calculator_dialog import run_raster_calculator_assistant

            launch_params = run_raster_calculator_assistant(
                self.iface,
                include_pro=self.provider.include_pro,
                tier=self.provider.tier,
            )
        except Exception as exc:
            self._notify_warning(
                f"Raster Calculator assistant unavailable; panel launch requires assistant ({exc})."
            )
            return

        if launch_params == {}:
            # Assistant was opened and then cancelled.
            return

        provider_id = self.provider.id()
        opened = open_processing_algorithm_dialog(
            self.iface,
            provider_id,
            tool_id,
            params=launch_params,
        )
        if opened:
            self._record_last_tool(tool_id)
            self._record_recent_tool(tool_id)
            if launch_params:
                self._notify_info(
                    "Opened Raster Calculator processing dialog with assistant parameters."
                )
            else:
                self._notify_info("Opening tool: raster_calculator")
        else:
            self._notify_warning(
                "Unable to open dialog for raster_calculator; host processing API not available."
            )

    def _open_tool_from_recent(self, tool_id: str):
        self._open_tool_from_panel(tool_id)

    def _open_tool_from_favorite(self, tool_id: str):
        self._open_tool_from_panel(tool_id)

    def _open_recipe_from_panel(self, recipe_id: str):
        recipe = self._recipe_index.get(str(recipe_id), {})
        launch_tool = str(recipe.get("launch_tool", "")).strip()
        if not launch_tool:
            self._notify_warning(f"Recipe is not launchable: {recipe_id}")
            return

        if bool(recipe.get("locked", False)):
            self._show_recipe_upgrade_from_panel(recipe_id)
            return

        provider_id = self.provider.id()
        opened = open_processing_algorithm_dialog(self.iface, provider_id, launch_tool)
        if not opened:
            self._notify_warning(
                f"Unable to open launch tool '{launch_tool}' for recipe '{recipe_id}'."
            )
            return

        self._record_last_tool(launch_tool)
        self._record_recent_tool(launch_tool)
        summary = str(recipe.get("summary", "")).strip()
        if summary:
            self._notify_info(f"Recipe '{recipe.get('title', recipe_id)}': {summary}")
        else:
            self._notify_info(f"Opened recipe launch tool: {launch_tool}")

    def _show_recipe_upgrade_from_panel(self, recipe_id: str):
        recipe = self._recipe_index.get(str(recipe_id), {})
        if not recipe:
            self._notify_warning(f"Recipe not found: {recipe_id}")
            return

        title = str(recipe.get("title", recipe_id)).strip() or recipe_id
        tier = str(recipe.get("tier", "open")).strip().upper() or "OPEN"
        reason = str(recipe.get("locked_reason", "This recipe is currently unavailable.")).strip()
        tools = [str(t) for t in recipe.get("tools", []) if str(t).strip()]
        steps = " -> ".join(tools) if tools else "(no defined steps)"

        if not bool(recipe.get("locked", False)):
            self._notify_info(f"Recipe '{title}' is available in the current runtime tier.")
            return

        message = (
            f"Recipe: {title}\n"
            f"Required tier: {tier}\n"
            f"Reason: {reason}\n\n"
            f"Workflow steps: {steps}\n\n"
            "Activate a runtime with the required tier and refresh the catalog."
        )

        try:
            if show_info_dialog(self.iface, "Whitebox Workflows Recipe Access", message):
                return
        except Exception:
            pass

        try:
            if show_info_dialog(None, "Whitebox Workflows Recipe Access", message):
                return
        except Exception:
            pass

        self._notify_warning(message)

    def _copy_recipe_steps_from_panel(self, recipe_id: str):
        recipe = self._recipe_index.get(str(recipe_id), {})
        if not recipe:
            self._notify_warning(f"Recipe not found: {recipe_id}")
            return

        text = recipe_steps_text(recipe)
        try:
            QApplication.clipboard().setText(text)
            self._notify_info(f"Copied recipe steps: {recipe.get('title', recipe_id)}")
            return
        except Exception:
            pass

        self._notify_warning("Unable to copy recipe steps to clipboard in this host.")

    def _open_recipe_file_from_panel(self):
        path = ensure_user_recipe_file()
        path_str = str(path)

        # Best-effort host/system open.
        opened = open_local_file(path_str)

        if not opened:
            try:
                QApplication.clipboard().setText(path_str)
            except Exception:
                pass
            self._notify_info(f"Recipe file: {path_str}")
            return

        self._notify_info(f"Opened recipe file: {path_str}")

    def _reload_recipe_file_from_panel(self):
        path = ensure_user_recipe_file()
        self._refresh_catalog(silent=True)
        self._notify_info(f"Reloaded recipes from: {path}")

    def _validate_recipe_file_from_panel(self):
        path = ensure_user_recipe_file()
        recipe_catalog, warnings = load_all_recipe_definitions()

        built_in_count = len([r for r in recipe_catalog if str(r.get("id", "")).strip()])
        if warnings:
            lines = [
                f"Recipe file: {path}",
                f"Validation status: {len(warnings)} warning(s)",
                f"Usable recipes loaded: {built_in_count}",
                "",
                "Warnings:",
            ]
            lines.extend([f"- {w}" for w in warnings])
            message = "\n".join(lines)
            try:
                if show_info_dialog(self.iface, "Whitebox Workflows Recipe Validation", message):
                    return
            except Exception:
                pass

            try:
                if show_info_dialog(None, "Whitebox Workflows Recipe Validation", message):
                    return
            except Exception:
                pass
            self._notify_warning(message)
            return

        message = (
            f"Recipe file: {path}\n"
            f"Validation status: OK\n"
            f"Usable recipes loaded: {built_in_count}"
        )
        try:
            if show_info_dialog(self.iface, "Whitebox Workflows Recipe Validation", message):
                return
        except Exception:
            pass

        try:
            if show_info_dialog(None, "Whitebox Workflows Recipe Validation", message):
                return
        except Exception:
            pass
        self._notify_info(message)

    def _record_recent_tool(self, tool_id: str):
        if tool_id in self._recent_tool_ids:
            self._recent_tool_ids.remove(tool_id)
        self._recent_tool_ids.insert(0, tool_id)
        if len(self._recent_tool_ids) > self._max_recent_tools:
            self._recent_tool_ids = self._recent_tool_ids[: self._max_recent_tools]
        if self._dock_panel is not None:
            self._dock_panel.set_recent_tools(self._recent_tool_ids)
        self._save_recent_tools()

    def _add_selected_favorite(self, *_args):
        if self._dock_panel is None:
            return
        tool_id = self._dock_panel.selected_result_tool_id()
        self._add_favorite_by_id(tool_id)

    def _toggle_selected_result_favorite(self, *_args):
        if self._dock_panel is None:
            return
        tool_id = self._dock_panel.selected_result_tool_id()
        if not tool_id:
            return
        if tool_id in self._favorite_tool_ids:
            self._remove_favorite_by_id(tool_id)
        else:
            self._add_favorite_by_id(tool_id)

    def _add_favorite_by_id(self, tool_id: str):
        if not tool_id:
            self._notify_warning("Select a tool in the results list to add a favorite.")
            return
        if tool_id in self._favorite_tool_ids:
            self._notify_info(f"Already a favorite: {tool_id}")
            return
        self._favorite_tool_ids.append(tool_id)
        self._save_favorite_tools()
        self._dock_panel.set_favorites(self._favorite_tool_ids)
        self._notify_info(f"Added favorite: {tool_id}")

    def _remove_selected_favorite(self, *_args):
        if self._dock_panel is None:
            return
        tool_id = self._dock_panel.selected_favorite_tool_id()
        self._remove_favorite_by_id(tool_id)

    def _remove_favorite_by_id(self, tool_id: str):
        if not tool_id:
            self._notify_warning("Select a favorite entry to remove.")
            return
        self._favorite_tool_ids = [t for t in self._favorite_tool_ids if t != tool_id]
        self._save_favorite_tools()
        self._dock_panel.set_favorites(self._favorite_tool_ids)
        self._notify_info(f"Removed favorite: {tool_id}")

    def _move_selected_favorite_up(self, *_args):
        if self._dock_panel is None:
            return
        idx = self._dock_panel.selected_favorite_index()
        if idx <= 0:
            return
        self._favorite_tool_ids[idx - 1], self._favorite_tool_ids[idx] = (
            self._favorite_tool_ids[idx],
            self._favorite_tool_ids[idx - 1],
        )
        self._save_favorite_tools()
        self._dock_panel.set_favorites(self._favorite_tool_ids)
        self._dock_panel.select_favorite_index(idx - 1)

    def _move_selected_favorite_down(self, *_args):
        if self._dock_panel is None:
            return
        idx = self._dock_panel.selected_favorite_index()
        if idx < 0 or idx >= len(self._favorite_tool_ids) - 1:
            return
        self._favorite_tool_ids[idx + 1], self._favorite_tool_ids[idx] = (
            self._favorite_tool_ids[idx],
            self._favorite_tool_ids[idx + 1],
        )
        self._save_favorite_tools()
        self._dock_panel.set_favorites(self._favorite_tool_ids)
        self._dock_panel.select_favorite_index(idx + 1)

    def _clear_favorites(self, *_args):
        self._favorite_tool_ids = []
        self._save_favorite_tools()
        if self._dock_panel is not None:
            self._dock_panel.set_favorites(self._favorite_tool_ids)
        self._notify_info("Cleared favorites.")

    def _clear_recents(self, *_args):
        self._recent_tool_ids = []
        self._save_recent_tools()
        if self._dock_panel is not None:
            self._dock_panel.set_recent_tools(self._recent_tool_ids)
        self._notify_info("Cleared recent tools.")

    def _on_quick_open_toggled(self, *_args):
        self._save_quick_open_preference()

    def _on_filter_state_changed(self, *_args):
        self._save_panel_ui_state()

    def _on_search_state_changed(self, *_args):
        self._save_panel_ui_state()

    def _on_recipe_discovery_state_changed(self, *_args):
        self._save_panel_ui_state()
        self._refresh_catalog(silent=True)

    def _on_focus_area_changed(self, *_args):
        self._save_panel_ui_state()

    def _show_tool_context_menu(self, source: str, tool_id: str, global_pos):
        menu = QMenu(self.iface.mainWindow())

        src = "Results" if source == "results" else "Favorites"
        title_action = menu.addAction(f"From: {src}")
        title_action.setEnabled(False)

        open_action = menu.addAction("Open Tool")
        if self._dock_panel is not None and self._dock_panel.is_favorite(tool_id):
            favorite_action = menu.addAction("Remove Favorite")
        else:
            favorite_action = menu.addAction("Add Favorite")
        copy_action = menu.addAction("Copy Tool ID")

        selected = run_menu(menu, global_pos)
        if selected is open_action:
            self._open_tool_from_panel(tool_id)
            return
        if selected is favorite_action:
            if self._dock_panel is not None and self._dock_panel.is_favorite(tool_id):
                self._remove_favorite_by_id(tool_id)
            else:
                self._add_favorite_by_id(tool_id)
            return
        if selected is copy_action:
            QApplication.clipboard().setText(tool_id)
            self._notify_info(f"Copied tool id: {tool_id}")

    def _load_recent_tools(self):
        try:
            settings = QSettings()
            raw = settings.value(self._settings_key_recent, "")
            if not raw:
                return
            parsed = json.loads(str(raw))
            if not isinstance(parsed, list):
                return
            cleaned = [str(x) for x in parsed if str(x).strip()]
            self._recent_tool_ids = cleaned[: self._max_recent_tools]
        except Exception:
            self._recent_tool_ids = []

    def _load_favorite_tools(self):
        try:
            settings = QSettings()
            raw = settings.value(self._settings_key_favorites, "")
            if raw is None or str(raw).strip() == "":
                self._has_persisted_favorites = False
                return
            self._has_persisted_favorites = True
            parsed = json.loads(str(raw))
            if not isinstance(parsed, list):
                return
            cleaned = [str(x) for x in parsed if str(x).strip()]
            self._favorite_tool_ids = cleaned
        except Exception:
            self._favorite_tool_ids = []
            self._has_persisted_favorites = False

    def _apply_catalog_display_defaults(self, catalog: list[dict]) -> None:
        if self._favorite_defaults_applied:
            return
        if self._has_persisted_favorites:
            self._favorite_defaults_applied = True
            return
        if self._favorite_tool_ids:
            self._favorite_defaults_applied = True
            return

        defaults: list[str] = []
        for item in catalog:
            if bool(item.get("display_default_favorite", False)):
                tool_id = str(item.get("id", "")).strip()
                if tool_id:
                    defaults.append(tool_id)

        # De-duplicate while preserving order from catalog sorting.
        deduped: list[str] = []
        seen = set()
        for tool_id in defaults:
            if tool_id in seen:
                continue
            seen.add(tool_id)
            deduped.append(tool_id)

        if deduped:
            self._favorite_tool_ids = deduped
            self._save_favorite_tools()

        self._favorite_defaults_applied = True

    def _apply_panel_tool_aliases(self, catalog: list[dict]) -> None:
        # Field Calculator has a dedicated assistant in the panel path; make the
        # panel label explicit so users can distinguish it from the generic
        # processing dialog entry.
        for item in catalog:
            tool_id = str(item.get("id", "")).strip().lower()
            if tool_id == "field_calculator":
                item["display_name"] = "Field Calculator Assistant"
                summary = str(item.get("summary", "")).strip()
                assistant_note = (
                    "Opens a guided assistant with expression snippets and preview "
                    "before launching the processing dialog."
                )
                if assistant_note not in summary:
                    if summary:
                        item["summary"] = f"{summary} {assistant_note}"
                    else:
                        item["summary"] = assistant_note
                continue

            if tool_id == "raster_calculator":
                item["display_name"] = "Raster Calculator Assistant"
                summary = str(item.get("summary", "")).strip()
                assistant_note = (
                    "Opens a guided assistant for raster expression authoring and "
                    "input ordering before launching the processing dialog."
                )
                if assistant_note not in summary:
                    if summary:
                        item["summary"] = f"{summary} {assistant_note}"
                    else:
                        item["summary"] = assistant_note

    def _load_quick_open_preference(self):
        try:
            settings = QSettings()
            raw = settings.value(self._settings_key_quick_open, True)
            if isinstance(raw, bool):
                self._quick_open_top_match = raw
            elif isinstance(raw, str):
                self._quick_open_top_match = raw.strip().lower() in ("1", "true", "yes", "on")
            else:
                self._quick_open_top_match = bool(raw)
        except Exception:
            self._quick_open_top_match = True

    def _load_runtime_preferences(self):
        try:
            settings = QSettings()
            include_exists = self._settings_contains(settings, self._settings_key_include_pro)
            tier_exists = self._settings_contains(settings, self._settings_key_requested_tier)

            if include_exists:
                self.provider.include_pro = self._coerce_bool(
                    settings.value(self._settings_key_include_pro, self.provider.include_pro),
                    self.provider.include_pro,
                )
            else:
                # Open-safe first-run default. Entitlement-based upgrade is
                # applied later once runtime capabilities are known.
                self.provider.include_pro = False

            default_requested_tier = "open"
            if tier_exists:
                tier_value = settings.value(self._settings_key_requested_tier, self.provider.tier)
            else:
                tier_value = default_requested_tier

            self.provider.tier = str(tier_value).strip().lower() or default_requested_tier
        except Exception:
            self.provider.include_pro = False
            self.provider.tier = self.provider.tier or "open"

    def _load_backend_preferences(self):
        try:
            settings = QSettings()
            mode = str(settings.value(self._settings_key_runtime_mode, "auto")).strip().lower()
            self._runtime_mode = mode if mode in ("auto", "local", "qgis") else "auto"
            self._runtime_local_python = str(settings.value(self._settings_key_local_python, "")).strip()
            self._auto_install_backend = self._coerce_bool(
                settings.value(self._settings_key_auto_install_backend, True),
                True,
            )
            self._auto_check_backend_updates = self._coerce_bool(
                settings.value(self._settings_key_auto_check_updates, True),
                True,
            )
            self._skip_auto_update_checks_in_local_mode = self._coerce_bool(
                settings.value(self._settings_key_skip_auto_update_checks_local_mode, True),
                True,
            )
            self._last_update_check_unix = int(
                float(settings.value(self._settings_key_last_update_check_unix, 0) or 0)
            )
            self._skipped_update_version = str(
                settings.value(self._settings_key_skipped_update_version, "")
            ).strip()
        except Exception:
            self._runtime_mode = "auto"
            self._runtime_local_python = ""
            self._auto_install_backend = True
            self._auto_check_backend_updates = True
            self._skip_auto_update_checks_in_local_mode = True
            self._last_update_check_unix = 0
            self._skipped_update_version = ""

    def _initialize_entitlement_runtime_defaults(self) -> None:
        """Set first-run runtime prefs from discovered runtime capabilities.

        This method only runs when no runtime preferences are persisted yet.
        It upgrades to Pro/Enterprise only when capabilities explicitly report
        that entitlement; otherwise defaults remain open-safe.
        """
        try:
            settings = QSettings()
            has_include = self._settings_contains(settings, self._settings_key_include_pro)
            has_tier = self._settings_contains(settings, self._settings_key_requested_tier)
            if has_include or has_tier:
                return
        except Exception:
            return

        try:
            payload = gather_runtime_diagnostics(include_pro=True, tier="pro")
            capabilities = payload.get("capabilities", {}) if isinstance(payload, dict) else {}
            effective_tier = str(capabilities.get("effective_tier", "")).strip().lower()

            if payload.get("status") == "ok" and effective_tier in {"pro", "enterprise"}:
                self.provider.include_pro = True
                self.provider.tier = effective_tier
            else:
                self.provider.include_pro = False
                self.provider.tier = "open"
        except Exception:
            self.provider.include_pro = False
            self.provider.tier = "open"

        self._save_runtime_preferences()

    def _save_backend_preferences(self):
        try:
            settings = QSettings()
            settings.setValue(self._settings_key_runtime_mode, self._runtime_mode)
            settings.setValue(self._settings_key_local_python, self._runtime_local_python)
            settings.setValue(self._settings_key_auto_install_backend, self._auto_install_backend)
            settings.setValue(self._settings_key_auto_check_updates, self._auto_check_backend_updates)
            settings.setValue(
                self._settings_key_skip_auto_update_checks_local_mode,
                self._skip_auto_update_checks_in_local_mode,
            )
            settings.setValue(self._settings_key_last_update_check_unix, int(self._last_update_check_unix))
            settings.setValue(self._settings_key_skipped_update_version, self._skipped_update_version)
        except Exception:
            pass

    def _apply_runtime_preferences_to_bootstrap(self):
        set_runtime_preferences(mode=self._runtime_mode, local_python=self._runtime_local_python)

    def _load_panel_ui_state(self):
        try:
            settings = QSettings()
            self._panel_visible = self._coerce_bool(
                settings.value(self._settings_key_panel_visible, True),
                True,
            )
            width_raw = settings.value(self._settings_key_panel_width, 320)
            self._panel_width = min(520, max(220, int(width_raw)))
            self._panel_show_available = self._coerce_bool(
                settings.value(self._settings_key_show_available, True),
                True,
            )
            self._panel_show_locked = self._coerce_bool(
                settings.value(self._settings_key_show_locked, True),
                True,
            )
            self._panel_show_locked_recipes = self._coerce_bool(
                settings.value(self._settings_key_show_locked_recipes, True),
                True,
            )
            self._panel_search_text = str(settings.value(self._settings_key_search_text, ""))
            self._panel_focus_area = str(settings.value(self._settings_key_focus_area, "search")).strip().lower() or "search"
        except Exception:
            self._panel_visible = True
            self._panel_width = 320
            self._panel_show_available = True
            self._panel_show_locked = True
            self._panel_show_locked_recipes = True
            self._panel_search_text = ""
            self._panel_focus_area = "search"

    def _load_last_tool(self):
        try:
            settings = QSettings()
            self._last_tool_id = str(settings.value(self._settings_key_last_tool, ""))
        except Exception:
            self._last_tool_id = ""

    def _record_last_tool(self, tool_id: str):
        self._last_tool_id = str(tool_id)
        try:
            settings = QSettings()
            settings.setValue(self._settings_key_last_tool, self._last_tool_id)
        except Exception:
            pass

    def _coerce_bool(self, value, default: bool) -> bool:
        if isinstance(value, bool):
            return value
        if isinstance(value, str):
            return value.strip().lower() in ("1", "true", "yes", "on")
        if value is None:
            return default
        return bool(value)

    def _settings_contains(self, settings, key: str) -> bool:
        contains = getattr(settings, "contains", None)
        if callable(contains):
            try:
                return bool(contains(key))
            except Exception:
                pass
        sentinel = "__WBW_MISSING__"
        try:
            return settings.value(key, sentinel) != sentinel
        except Exception:
            return False

    def _save_recent_tools(self):
        try:
            settings = QSettings()
            settings.setValue(self._settings_key_recent, json.dumps(self._recent_tool_ids))
        except Exception:
            pass

    def _save_favorite_tools(self):
        try:
            settings = QSettings()
            settings.setValue(self._settings_key_favorites, json.dumps(self._favorite_tool_ids))
            self._has_persisted_favorites = True
        except Exception:
            pass

    def _save_quick_open_preference(self):
        try:
            if self._dock_panel is not None:
                self._quick_open_top_match = self._dock_panel.quick_open_enabled()
            settings = QSettings()
            settings.setValue(self._settings_key_quick_open, self._quick_open_top_match)
        except Exception:
            pass

    def _save_runtime_preferences(self):
        try:
            settings = QSettings()
            settings.setValue(self._settings_key_include_pro, self.provider.include_pro)
            settings.setValue(self._settings_key_requested_tier, self.provider.tier)
        except Exception:
            pass

    def _save_panel_ui_state(self):
        try:
            panel = self._dock_panel
            if panel is not None:
                is_visible = getattr(panel, "isVisible", None)
                if callable(is_visible):
                    self._panel_visible = bool(is_visible())
                width = getattr(panel, "width", None)
                if callable(width):
                    self._panel_width = min(520, max(220, int(width())))
                self._panel_show_available = panel.show_available_enabled()
                self._panel_show_locked = panel.show_locked_enabled()
                self._panel_show_locked_recipes = panel.show_locked_recipes_enabled()
                self._panel_search_text = panel.search_text()
                self._panel_focus_area = panel.focus_area()
            settings = QSettings()
            settings.setValue(self._settings_key_panel_visible, self._panel_visible)
            settings.setValue(self._settings_key_panel_width, self._panel_width)
            settings.setValue(self._settings_key_show_available, self._panel_show_available)
            settings.setValue(self._settings_key_show_locked, self._panel_show_locked)
            settings.setValue(self._settings_key_show_locked_recipes, self._panel_show_locked_recipes)
            settings.setValue(self._settings_key_search_text, self._panel_search_text)
            settings.setValue(self._settings_key_focus_area, self._panel_focus_area)
        except Exception:
            pass

    def _toggle_panel(self, *_args):
        panel = self._dock_panel
        if panel is None:
            return
        is_visible = getattr(panel, "isVisible", None)
        set_visible = getattr(panel, "setVisible", None)
        if callable(is_visible) and callable(set_visible):
            set_visible(not bool(is_visible()))
            self._save_panel_ui_state()

    def _show_settings(self, *_args):
        current = WhiteboxPluginSettings(
            include_pro=self.provider.include_pro,
            tier=self.provider.tier,
            quick_open_top_match=self._quick_open_top_match,
            panel_show_available=self._panel_show_available,
            panel_show_locked=self._panel_show_locked,
            panel_show_locked_recipes=self._panel_show_locked_recipes,
            panel_width=self._panel_width,
            runtime_mode=self._runtime_mode,
            local_python_path=self._runtime_local_python,
            auto_install_backend=self._auto_install_backend,
            auto_check_backend_updates=self._auto_check_backend_updates,
            skip_auto_update_checks_in_local_mode=self._skip_auto_update_checks_in_local_mode,
        )
        try:
            dlg = WhiteboxSettingsDialog(current, parent=self.iface.mainWindow())
        except Exception:
            return
        run_dialog(dlg)
        if not dlg.was_accepted():
            return
        updated = dlg.read_settings()
        # Apply panel-side preferences immediately.
        self._quick_open_top_match = updated.quick_open_top_match
        self._panel_show_available = updated.panel_show_available
        self._panel_show_locked = updated.panel_show_locked
        self._panel_show_locked_recipes = updated.panel_show_locked_recipes
        self._panel_width = updated.panel_width
        if self._dock_panel is not None:
            self._dock_panel.set_quick_open_enabled(updated.quick_open_top_match)
            self._dock_panel.set_show_available_enabled(updated.panel_show_available)
            self._dock_panel.set_show_locked_enabled(updated.panel_show_locked)
            self._dock_panel.set_show_locked_recipes_enabled(updated.panel_show_locked_recipes)
            resize = getattr(self._dock_panel, "resize", None)
            if callable(resize):
                height_fn = getattr(self._dock_panel, "height", None)
                h = int(height_fn()) if callable(height_fn) else 600
                resize(updated.panel_width, h)
        # Save panel state.
        self._save_quick_open_preference()
        self._save_panel_ui_state()
        # Apply runtime discovery preferences; refresh catalog if they changed.
        runtime_changed = any(
            (
                updated.include_pro != self.provider.include_pro,
                updated.tier != self.provider.tier,
                updated.runtime_mode != self._runtime_mode,
                updated.local_python_path != self._runtime_local_python,
            )
        )
        backend_policy_changed = any(
            (
                updated.auto_install_backend != self._auto_install_backend,
                updated.auto_check_backend_updates != self._auto_check_backend_updates,
                updated.skip_auto_update_checks_in_local_mode != self._skip_auto_update_checks_in_local_mode,
            )
        )

        self._runtime_mode = updated.runtime_mode
        self._runtime_local_python = updated.local_python_path
        self._auto_install_backend = updated.auto_install_backend
        self._auto_check_backend_updates = updated.auto_check_backend_updates
        self._skip_auto_update_checks_in_local_mode = updated.skip_auto_update_checks_in_local_mode
        self._save_backend_preferences()
        self._apply_runtime_preferences_to_bootstrap()

        if runtime_changed:
            self.provider.include_pro = updated.include_pro
            self.provider.tier = updated.tier
            self._save_runtime_preferences()
            if self._ensure_backend_available(interactive=True):
                self._refresh_catalog()
            return

        if backend_policy_changed:
            self._notify_info("Backend install/update preferences saved.")

    def _manual_check_backend_updates(self, *_args):
        self._check_backend_updates(manual=True)

    def _confirm(self, title: str, message: str) -> bool:
        try:
            button_type = getattr(QMessageBox, "StandardButton", None)
            if button_type is not None:
                yes_btn = getattr(button_type, "Yes", None)
                no_btn = getattr(button_type, "No", None)
            else:
                yes_btn = getattr(QMessageBox, "Yes", None)
                no_btn = getattr(QMessageBox, "No", None)

            question = getattr(QMessageBox, "question", None)
            if callable(question) and yes_btn is not None and no_btn is not None:
                choice = question(self.iface.mainWindow(), title, message, yes_btn | no_btn, no_btn)
                return bool(choice == yes_btn)
        except Exception:
            pass

        # Non-interactive fallback for test hosts.
        return False

    def _prompt_backend_update_action(self, title: str, message: str) -> str:
        """Return one of: update, remind, skip, cancel."""
        try:
            msg_box = QMessageBox(self.iface.mainWindow())
            set_title = getattr(msg_box, "setWindowTitle", None)
            if callable(set_title):
                set_title(title)
            set_text = getattr(msg_box, "setText", None)
            if callable(set_text):
                set_text(message)

            add_button = getattr(msg_box, "addButton", None)
            if not callable(add_button):
                return "cancel"

            button_role = getattr(QMessageBox, "ButtonRole", None)
            accept_role = getattr(button_role, "AcceptRole", None) if button_role is not None else None
            reject_role = getattr(button_role, "RejectRole", None) if button_role is not None else None
            destructive_role = (
                getattr(button_role, "DestructiveRole", None) if button_role is not None else None
            )

            update_btn = add_button("Update Now", accept_role if accept_role is not None else 0)
            remind_btn = add_button("Remind Later", reject_role if reject_role is not None else 1)
            skip_btn = add_button("Skip This Version", destructive_role if destructive_role is not None else 2)

            set_default = getattr(msg_box, "setDefaultButton", None)
            if callable(set_default):
                set_default(update_btn)

            run_dialog(msg_box)

            clicked = getattr(msg_box, "clickedButton", None)
            clicked_button = clicked() if callable(clicked) else None
            if clicked_button is update_btn:
                return "update"
            if clicked_button is remind_btn:
                return "remind"
            if clicked_button is skip_btn:
                return "skip"
            return "cancel"
        except Exception:
            return "cancel"

    def _ensure_backend_available(self, interactive: bool) -> bool:
        try:
            status = backend_install_status()
        except Exception as exc:
            self._notify_warning(f"Unable to inspect whitebox_workflows backend: {exc}")
            return False

        installed = str(status.get("installed_version", "")).strip()
        if installed:
            return True

        if not interactive:
            return False

        self._show_backend_install_instructions()
        return False

    def _build_install_command(self) -> str:
        """Return a single-line exec() command safe to paste into the QGIS Python Console.

        Installs whitebox-workflows via pip then reloads the plugin in-place,
        so the user never needs to restart QGIS.
        """
        import os
        if os.name == "nt":
            pip_args = "['pip','install','whitebox-workflows']"
        else:
            pip_args = "['pip','install','--user','whitebox-workflows']"

        inner = (
            "import runpy,sys\\n"
            f"sys.argv={pip_args}\\n"
            "try:\\n"
            "    runpy.run_module('pip',run_name='__main__',alter_sys=True)\\n"
            "except SystemExit:\\n"
            "    pass\\n"
            "import qgis.utils\\n"
            "qgis.utils.unloadPlugin('whitebox_workflows_qgis')\\n"
            "qgis.utils.loadPlugin('whitebox_workflows_qgis')\\n"
            "qgis.utils.startPlugin('whitebox_workflows_qgis')\\n"
            "print('Whitebox Workflows backend installed and plugin reloaded.')"
        )
        return f'exec("{inner}")'

    def _show_backend_install_instructions(self) -> None:
        """Show a dialog with step-by-step installation instructions."""
        import os

        command = self._build_install_command()

        if os.name == "nt":
            console_path = "Plugins  ▸  Python Console"
            alt_note = "Alternatively, open the OSGeo4W Shell and run:\n  python -m pip install whitebox-workflows"
        else:
            console_path = "Plugins  ▸  Python Console"
            alt_note = ""

        try:
            from qgis.PyQt.QtWidgets import (
                QDialog, QVBoxLayout, QHBoxLayout,
                QLabel, QPlainTextEdit, QPushButton,
            )
            from qgis.PyQt.QtGui import QFont

            dlg = QDialog(self.iface.mainWindow())
            dlg.setWindowTitle("Whitebox Workflows — Backend Not Installed")
            dlg.setMinimumWidth(620)

            layout = QVBoxLayout(dlg)
            layout.setSpacing(10)

            # Instructions
            instructions = (
                "<b>The whitebox-workflows backend is not installed.</b><br><br>"
                "To install it:<ol>"
                f"<li>Open the QGIS Python Console:<br>&nbsp;&nbsp;&nbsp;<b>{console_path}</b></li>"
                "<li>Paste the command below into the console and press <b>Enter</b>.</li>"
                "<li>The backend will install and the plugin will reload automatically — "
                "no restart needed.</li>"
                "</ol>"
            )
            if alt_note:
                instructions += f"<br><i>{alt_note}</i>"

            lbl = QLabel(instructions)
            lbl.setWordWrap(True)
            lbl.setOpenExternalLinks(False)
            layout.addWidget(lbl)

            # Command label
            layout.addWidget(QLabel("<b>Command to paste in the Python Console:</b>"))

            # Command box
            cmd_edit = QPlainTextEdit(command)
            cmd_edit.setReadOnly(True)
            font = QFont()
            font.setFamily("Courier New" if os.name == "nt" else "Menlo")
            font.setPointSize(10)
            cmd_edit.setFont(font)
            cmd_edit.setFixedHeight(140)
            layout.addWidget(cmd_edit)

            # Button row
            btn_row = QHBoxLayout()

            copy_btn = QPushButton("Copy command to clipboard")
            def _copy():
                try:
                    from qgis.PyQt.QtWidgets import QApplication
                    QApplication.clipboard().setText(command)
                    copy_btn.setText("✓ Copied!")
                except Exception:
                    pass
            copy_btn.clicked.connect(_copy)
            btn_row.addWidget(copy_btn)
            btn_row.addStretch()

            close_btn = QPushButton("Close")
            close_btn.setDefault(True)
            close_btn.clicked.connect(dlg.accept)
            btn_row.addWidget(close_btn)

            layout.addLayout(btn_row)

            dlg.exec() if hasattr(dlg, "exec") and callable(dlg.exec) else dlg.exec_()

        except Exception as exc:
            # Last-resort: plain notification. Sanitise to one readable line.
            self._notify_warning(
                "whitebox-workflows is not installed. "
                f"Open the QGIS Python Console ({console_path}) and run: "
                "import runpy, sys; sys.argv=['pip','install','--user','whitebox-workflows']; "
                "runpy.run_module('pip', run_name='__main__', alter_sys=True)"
            )

    def _main_window_or_none(self):
        try:
            return self.iface.mainWindow()
        except Exception:
            return None

    def _check_backend_updates(self, manual: bool = False):
        if not manual:
            if not self._auto_check_backend_updates:
                return
            if self._runtime_mode == "local" and self._skip_auto_update_checks_in_local_mode:
                return
            now_unix = int(datetime.now().timestamp())
            check_interval_seconds = 24 * 60 * 60
            if self._last_update_check_unix > 0 and (now_unix - self._last_update_check_unix) < check_interval_seconds:
                return

        try:
            status = backend_update_status()
        except Exception as exc:
            if manual:
                self._notify_warning(f"Backend update check failed: {exc}")
            return

        self._last_update_check_unix = int(datetime.now().timestamp())
        self._save_backend_preferences()

        error_text = str(status.get("error", "")).strip()
        if error_text:
            if manual:
                self._notify_warning(f"Backend update check failed: {error_text}")
            return

        latest = str(status.get("latest_version", "")).strip()
        installed = str(status.get("installed_version", "")).strip()
        interpreter = str(status.get("interpreter", "")).strip()
        update_available = bool(status.get("update_available", False))

        if not update_available:
            if manual:
                if installed:
                    self._notify_info(f"whitebox_workflows is up to date ({installed}).")
                else:
                    self._notify_warning("whitebox_workflows is not currently installed.")
            return

        if latest and latest == self._skipped_update_version and not manual:
            return

        prompt = (
            "A newer whitebox_workflows backend is available.\n\n"
            f"Installed: {installed or '(not installed)'}\n"
            f"Latest: {latest or 'unknown'}\n"
            f"Interpreter: {interpreter or 'unknown'}\n\n"
            "Choose an action below."
        )
        action = self._prompt_backend_update_action("Update Whitebox Workflows Backend", prompt)
        if action in ("cancel", "remind"):
            if manual:
                self._notify_info("Backend update deferred.")
            return

        if action == "skip":
            if latest:
                self._skipped_update_version = latest
                self._save_backend_preferences()
            if manual:
                self._notify_info("Backend update skipped for this version.")
            return

        if action != "update":
            return

        try:
            result = install_or_upgrade_whitebox_workflows(upgrade=True)
            version = str(result.get("installed_version", "")).strip() or latest
            self._skipped_update_version = ""
            self._save_backend_preferences()
            self._notify_info(f"Updated whitebox_workflows to {version}.")
            self._refresh_catalog(silent=True)
        except Exception as exc:
            self._notify_warning(f"Backend update failed: {exc}")

    def _show_diagnostics(self, *_args):
        payload = gather_runtime_diagnostics(
            include_pro=self.provider.include_pro,
            tier=self.provider.tier,
        )
        text = diagnostics_text(payload)
        update_policy = (
            f"Backend update policy:\n"
            f"  runtime_mode: {self._runtime_mode}\n"
            f"  auto_check_backend_updates: {self._auto_check_backend_updates}\n"
            f"  skip_auto_update_checks_in_local_mode: {self._skip_auto_update_checks_in_local_mode}"
        )
        text = f"{text}\n\n{update_policy}"

        try:
            if show_info_dialog(self.iface, "Whitebox Workflows Diagnostics", text):
                return
        except Exception:
            pass

        try:
            if show_info_dialog(None, "Whitebox Workflows Diagnostics", text):
                return
        except Exception:
            pass

        push_host_message(
            self.iface,
            "Whitebox Workflows",
            "Diagnostics unavailable as dialog; see logs.",
            level="warning",
        )

    def _refresh_catalog(self, *_args, silent: bool = False):
        # Pull recents from persisted settings so tools run from the Processing
        # toolbox also appear in panel recents.
        self._load_recent_tools()
        try:
            catalog = self.provider.refresh_catalog(regenerate_help=True)
            refresh_algorithms = getattr(self.provider, "refreshAlgorithms", None)
            if callable(refresh_algorithms):
                refresh_algorithms()
        except Exception as exc:
            self._notify_warning(f"Catalog refresh failed: {exc}")
            return

        available, locked = summarize_catalog(catalog)
        self._apply_panel_tool_aliases(catalog)
        self._apply_catalog_display_defaults(catalog)

        payload = gather_runtime_diagnostics(
            include_pro=self.provider.include_pro,
            tier=self.provider.tier,
        )
        caps = payload.get("capabilities") if isinstance(payload, dict) else None
        effective_tier = "unknown"
        if isinstance(caps, dict):
            effective_tier = str(caps.get("effective_tier", "unknown"))

        downgrade_warning = self._build_pro_downgrade_warning(caps)
        expiry_notice, expiry_urgent = self._build_license_expiry_notice(caps, effective_tier)

        if self._dock_panel is not None:
            self._dock_panel.set_catalog(catalog)
            self._dock_panel.set_favorites(self._favorite_tool_ids)
            self._dock_panel.set_recent_tools(self._recent_tool_ids)

            recipe_catalog, recipe_warnings = load_all_recipe_definitions()

            recipes = visible_recipes(
                effective_tier=effective_tier,
                catalog=catalog,
                include_locked_discovery=self._panel_show_locked_recipes,
                recipe_catalog=recipe_catalog,
            )
            self._recipe_index = {
                str(item.get("id", "")): item
                for item in recipes
                if str(item.get("id", "")).strip()
            }
            self._dock_panel.set_recipes(recipes)

            if recipe_warnings:
                self._notify_warning(
                    f"Recipe file issues ({user_recipe_file_path()}): {recipe_warnings[0]}"
                )

            if self._last_tool_id:
                self._dock_panel.select_result_by_tool_id(self._last_tool_id)
                self._dock_panel.select_favorite_by_tool_id(self._last_tool_id)
            refreshed_at = datetime.now().strftime("%Y-%m-%d %H:%M:%S")
            raw_detail = str(payload.get("error", ""))
            status_detail = " ".join(raw_detail.split())
            if len(status_detail) > 240:
                status_detail = f"{status_detail[:237]}..."

            detail_parts = []
            if status_detail:
                detail_parts.append(status_detail)
            if downgrade_warning:
                detail_parts.append(downgrade_warning)
            if expiry_notice:
                detail_parts.append(expiry_notice)
            banner_detail = " | ".join(detail_parts)

            self._dock_panel.update_session_banner(
                status=str(payload.get("status", "unknown")),
                effective_tier=effective_tier,
                visible_count=available + locked,
                refreshed_at=refreshed_at,
                detail=banner_detail,
            )
            self._dock_panel.update_state(
                status=str(payload.get("status", "unknown")),
                fallback_tier=self.provider.tier,
                effective_tier=effective_tier,
                available_count=available,
                locked_count=locked,
                qgis_version=qgis_version_string(),
            )

        warning_parts = []
        if downgrade_warning:
            warning_parts.append(downgrade_warning)
        if expiry_urgent and expiry_notice:
            warning_parts.append(expiry_notice)

        warning_signature = " || ".join(warning_parts)
        if warning_signature and warning_signature != self._last_runtime_warning_signature:
            self._notify_warning(warning_signature)
            self._last_runtime_warning_signature = warning_signature
        elif not warning_signature:
            self._last_runtime_warning_signature = ""

        if not silent:
            self._notify_info(
                f"Catalog refreshed: {available} available, {locked} locked tools."
            )

    def _notify_info(self, message: str):
        push_host_message(self.iface, "Whitebox Workflows", message, level="info")

    def _notify_warning(self, message: str):
        push_host_message(self.iface, "Whitebox Workflows", message, level="warning")

    def _get_capability_value(self, capabilities, *keys):
        if not isinstance(capabilities, dict):
            return None
        for key in keys:
            if key in capabilities:
                return capabilities.get(key)

        entitlement = capabilities.get("entitlement")
        if isinstance(entitlement, dict):
            for key in keys:
                if key in entitlement:
                    return entitlement.get(key)
        return None

    def _build_pro_downgrade_warning(self, capabilities) -> str:
        if not isinstance(capabilities, dict):
            return ""

        if not bool(self.provider.include_pro):
            return ""

        compiled_with_pro = self._get_capability_value(capabilities, "compiled_with_pro_support")
        runtime_include_pro = self._get_capability_value(capabilities, "include_pro")

        if runtime_include_pro is False:
            return (
                "Pro catalog unavailable: runtime downgraded to open mode (include_pro=false). "
                "Open tools remain available; refresh the catalog after any runtime change."
            )
        if compiled_with_pro is False:
            return (
                "Pro catalog unavailable: active whitebox_workflows build does not include Pro support. "
                "Reinstall wbw_python with --pro and refresh catalog."
            )
        return ""

    def _build_license_expiry_notice(self, capabilities, effective_tier: str) -> tuple[str, bool]:
        if not isinstance(capabilities, dict):
            return "", False

        tier = str(effective_tier or "").strip().lower()
        if tier not in {"pro", "enterprise"}:
            return "", False

        expiry_raw = self._get_capability_value(
            capabilities,
            "entitlement_expires_at_unix",
            "license_expires_at_unix",
            "expires_at_unix",
            "expiry_unix",
        )
        if expiry_raw is None:
            return "", False

        try:
            expiry_unix = int(float(expiry_raw))
        except Exception:
            return "", False

        now_unix = int(datetime.now().timestamp())
        remaining = expiry_unix - now_unix
        warn_window_seconds = 45 * 24 * 60 * 60

        if remaining <= 0:
            when = datetime.fromtimestamp(expiry_unix).strftime("%Y-%m-%d")
            return f"License expired on {when}. Pro tools may be unavailable until renewed.", True

        if remaining <= warn_window_seconds:
            days = int((remaining + 86399) // 86400)
            when = datetime.fromtimestamp(expiry_unix).strftime("%Y-%m-%d")
            return f"License expires in {days} day(s) on {when}.", True

        return "", False

    def unload(self):
        self._save_recent_tools()
        self._save_favorite_tools()
        self._save_quick_open_preference()
        self._save_panel_ui_state()
        self._save_backend_preferences()
        if self._dock_panel is not None:
            unregister_dock_widget(self.iface, self._dock_panel)
            self._dock_panel = None
        if self._panel_action is not None:
            unregister_plugin_action(self.iface, self._panel_action, self._menu_label)
            self._panel_action = None
        if self._refresh_action is not None:
            unregister_plugin_action(self.iface, self._refresh_action, self._menu_label)
            self._refresh_action = None
        if self._diagnostics_action is not None:
            unregister_plugin_action(self.iface, self._diagnostics_action, self._menu_label)
            self._diagnostics_action = None
        if self._settings_action is not None:
            unregister_plugin_action(self.iface, self._settings_action, self._menu_label)
            self._settings_action = None
        if self._activate_license_action is not None:
            unregister_plugin_action(self.iface, self._activate_license_action, self._menu_label)
            self._activate_license_action = None
        if self._deactivate_license_action is not None:
            unregister_plugin_action(self.iface, self._deactivate_license_action, self._menu_label)
            self._deactivate_license_action = None
        if self._transfer_license_action is not None:
            unregister_plugin_action(self.iface, self._transfer_license_action, self._menu_label)
            self._transfer_license_action = None
        if self._backend_updates_action is not None:
            unregister_plugin_action(self.iface, self._backend_updates_action, self._menu_label)
            self._backend_updates_action = None
        if not self._provider_registered:
            return
        if unregister_provider(self.iface, self.provider):
            self._provider_registered = False
