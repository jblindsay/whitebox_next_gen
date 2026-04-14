from __future__ import annotations

from .provider import WhiteboxProcessingProvider


class WhiteboxWorkflowsPlugin:
    def __init__(self, iface):
        self.iface = iface
        self.provider = WhiteboxProcessingProvider()

    def initGui(self):
        registry_getter = getattr(self.iface, "processingRegistry", None)
        if registry_getter is None:
            return
        registry = registry_getter()
        add_provider = getattr(registry, "addProvider", None)
        if add_provider is not None:
            add_provider(self.provider)

    def unload(self):
        registry_getter = getattr(self.iface, "processingRegistry", None)
        if registry_getter is None:
            return
        registry = registry_getter()
        remove_provider = getattr(registry, "removeProvider", None)
        if remove_provider is not None:
            remove_provider(self.provider)