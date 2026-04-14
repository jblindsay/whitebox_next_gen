def classFactory(iface):
    from .plugin import WhiteboxWorkflowsPlugin

    return WhiteboxWorkflowsPlugin(iface)