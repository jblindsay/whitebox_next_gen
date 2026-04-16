from __future__ import annotations


def qgis_version_string() -> str:
    """Return the host QGIS version string if available."""
    try:
        from qgis.core import Qgis  # type: ignore[import]

        value = getattr(Qgis, "QGIS_VERSION", "")
        return str(value) if value else ""
    except Exception:
        return ""


def qgis_major_version() -> int:
    """Return host QGIS major version, or 0 when unknown."""
    v = qgis_version_string()
    if not v:
        return 0
    try:
        return int(v.split(".", 1)[0])
    except Exception:
        return 0


def _registry_from_iface(iface):
    getter = getattr(iface, "processingRegistry", None)
    if getter is None:
        return None
    try:
        return getter()
    except Exception:
        return None


def _registry_from_app():
    try:
        from qgis.core import QgsApplication  # type: ignore[import]

        getter = getattr(QgsApplication, "processingRegistry", None)
        if getter is None:
            return None
        return getter()
    except Exception:
        return None


def resolve_processing_registry(iface):
    """Resolve processing registry with QGIS 4-first fallbacks.

    Resolution order:
      1) iface.processingRegistry()
      2) QgsApplication.processingRegistry()
    """
    registry = _registry_from_iface(iface)
    if registry is not None:
        return registry
    return _registry_from_app()


def register_provider(iface, provider) -> bool:
    """Register a processing provider; return True on success."""
    registry = resolve_processing_registry(iface)
    if registry is None:
        return False

    add_provider = getattr(registry, "addProvider", None)
    if add_provider is None:
        return False

    try:
        add_provider(provider)
        return True
    except Exception:
        return False


def unregister_provider(iface, provider) -> bool:
    """Unregister a processing provider; return True on success."""
    registry = resolve_processing_registry(iface)
    if registry is None:
        return False

    remove_provider = getattr(registry, "removeProvider", None)
    if remove_provider is None:
        return False

    try:
        remove_provider(provider)
        return True
    except Exception:
        return False


def register_plugin_action(iface, action, menu_label: str) -> bool:
    """Register a plugin QAction in host menu/toolbar with safe fallbacks."""
    registered = False

    add_to_menu = getattr(iface, "addPluginToMenu", None)
    if add_to_menu is not None:
        try:
            add_to_menu(menu_label, action)
            registered = True
        except Exception:
            pass

    add_toolbar_icon = getattr(iface, "addToolBarIcon", None)
    if add_toolbar_icon is not None:
        try:
            add_toolbar_icon(action)
            registered = True
        except Exception:
            pass

    if not registered:
        # QGIS 4 host fallback: add directly to plugin menu if exposed.
        plugin_menu_getter = getattr(iface, "pluginMenu", None)
        if plugin_menu_getter is not None:
            try:
                plugin_menu = plugin_menu_getter()
                if plugin_menu is not None:
                    plugin_menu.addAction(action)
                    registered = True
            except Exception:
                pass

    if not registered:
        # Last-resort shortcut visibility via main-window action list.
        main_window = getattr(iface, "mainWindow", None)
        if main_window is not None:
            try:
                mw = main_window()
                if mw is not None and hasattr(mw, "addAction"):
                    mw.addAction(action)
                    registered = True
            except Exception:
                pass

    return registered


def unregister_plugin_action(iface, action, menu_label: str) -> bool:
    """Unregister a plugin QAction from host menu/toolbar with safe fallbacks."""
    removed = False

    remove_from_menu = getattr(iface, "removePluginMenu", None)
    if remove_from_menu is not None:
        try:
            remove_from_menu(menu_label, action)
            removed = True
        except Exception:
            pass

    remove_toolbar_icon = getattr(iface, "removeToolBarIcon", None)
    if remove_toolbar_icon is not None:
        try:
            remove_toolbar_icon(action)
            removed = True
        except Exception:
            pass

    plugin_menu_getter = getattr(iface, "pluginMenu", None)
    if plugin_menu_getter is not None:
        try:
            plugin_menu = plugin_menu_getter()
            remove_action = getattr(plugin_menu, "removeAction", None) if plugin_menu is not None else None
            if callable(remove_action):
                remove_action(action)
                removed = True
        except Exception:
            pass

    main_window = getattr(iface, "mainWindow", None)
    if main_window is not None:
        try:
            mw = main_window()
            remove_action = getattr(mw, "removeAction", None) if mw is not None else None
            if callable(remove_action):
                remove_action(action)
                removed = True
        except Exception:
            pass

    return removed


def register_dock_widget(iface, dock) -> bool:
    """Register plugin dock widget in QGIS UI with safe fallbacks."""
    try:
        from qgis.PyQt.QtCore import Qt  # type: ignore[import]

        area = getattr(Qt, "RightDockWidgetArea", None)
        if area is None:
            dock_enum = getattr(Qt, "DockWidgetArea", None)
            area = getattr(dock_enum, "RightDockWidgetArea", None)
        if area is None:
            area = getattr(Qt, "LeftDockWidgetArea", None)
        if area is None:
            dock_enum = getattr(Qt, "DockWidgetArea", None)
            area = getattr(dock_enum, "LeftDockWidgetArea", None)
    except Exception:
        area = None

    add_dock = getattr(iface, "addDockWidget", None)
    if add_dock is None:
        return False

    try:
        if area is None:
            add_dock(dock)
        else:
            add_dock(area, dock)
        return True
    except Exception:
        return False


def unregister_dock_widget(iface, dock) -> bool:
    """Remove plugin dock widget from QGIS UI with safe fallbacks."""
    remove_dock = getattr(iface, "removeDockWidget", None)
    if remove_dock is None:
        return False
    try:
        remove_dock(dock)
        return True
    except Exception:
        return False


def open_processing_algorithm_dialog(iface, provider_id: str, tool_id: str) -> bool:
    """Open a processing algorithm dialog with API fallbacks.

    Returns True if any known host API successfully opens a dialog.
    """
    full_id = f"{provider_id}:{tool_id}"

    # Resolve the algorithm object up front so we can avoid ambiguous short-id
    # routing in hosts/providers where similarly named tools exist.
    alg_obj = None
    try:
        registry = resolve_processing_registry(iface)
        by_id = getattr(registry, "algorithmById", None) if registry is not None else None
        if callable(by_id):
            alg_obj = by_id(full_id)
    except Exception:
        alg_obj = None

    # Prefer interface methods that open a runnable processing dialog.
    candidates = [
        ("openProcessingAlgorithmDialog", (full_id, {})),
        ("showProcessingAlgorithmDialog", (full_id, {})),
        ("openProcessingAlgorithmDialog", (full_id,)),
        ("showProcessingAlgorithmDialog", (full_id,)),
    ]

    if alg_obj is not None:
        candidates.extend(
            [
                ("openProcessingAlgorithmDialog", (alg_obj, {})),
                ("showProcessingAlgorithmDialog", (alg_obj, {})),
                ("openProcessingAlgorithmDialog", (alg_obj,)),
                ("showProcessingAlgorithmDialog", (alg_obj,)),
            ]
        )

    for method_name, args in candidates:
        method = getattr(iface, method_name, None)
        if method is None:
            continue
        try:
            method(*args)
            return True
        except TypeError:
            continue
        except Exception:
            continue

    # Processing plugin/module API fallbacks for hosts that do not expose
    # dialog helpers directly on iface.
    try:
        import processing  # type: ignore[import]

        processing_candidates = [
            # execAlgorithmDialog tends to create a fully wired run flow.
            ("execAlgorithmDialog", (full_id, {})),
            ("showAlgorithmDialog", (full_id, {})),
            ("execAlgorithmDialog", (full_id,)),
            ("showAlgorithmDialog", (full_id,)),
        ]

        for method_name, args in processing_candidates:
            method = getattr(processing, method_name, None)
            if method is None:
                continue
            try:
                method(*args)
                return True
            except TypeError:
                continue
            except Exception:
                continue
    except Exception:
        pass

    return False
