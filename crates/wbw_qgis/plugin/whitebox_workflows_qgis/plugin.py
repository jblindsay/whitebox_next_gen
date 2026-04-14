from __future__ import annotations

from .host_api import qgis_major_version, qgis_version_string, register_provider, unregister_provider
from .provider import WhiteboxProcessingProvider


class WhiteboxWorkflowsPlugin:
    def __init__(self, iface):
        self.iface = iface
        self.provider = WhiteboxProcessingProvider()
        self._provider_registered = False

    def initGui(self):
        # QGIS 4 is the primary target; avoid hard-fail in unknown hosts.
        major = qgis_major_version()
        if major not in (0, 4):
            return

        if not register_provider(self.iface, self.provider):
            return
        self._provider_registered = True

        # Helpful startup message where the host exposes a message bar.
        msg = f"Whitebox Workflows provider loaded (QGIS {qgis_version_string() or 'unknown'})."
        try:
            bar = self.iface.messageBar()
            push = getattr(bar, "pushInfo", None)
            if push is not None:
                push("Whitebox Workflows", msg)
        except Exception:
            pass

    def unload(self):
        if not self._provider_registered:
            return
        if unregister_provider(self.iface, self.provider):
            self._provider_registered = False