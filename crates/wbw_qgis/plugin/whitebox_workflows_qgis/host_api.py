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

    return removed
