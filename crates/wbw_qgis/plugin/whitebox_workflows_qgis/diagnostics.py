from __future__ import annotations

import json
from typing import Any

from .bootstrap import RuntimeBootstrapError, get_runtime_capabilities


def gather_runtime_diagnostics(include_pro: bool = True, tier: str = "open") -> dict[str, Any]:
    """Collect runtime diagnostics for the plugin environment.

    The returned payload is designed to be stable and easy to render in a
    message dialog or logs.
    """
    payload: dict[str, Any] = {
        "status": "ok",
        "include_pro": include_pro,
        "fallback_tier": tier,
    }

    try:
        capabilities = get_runtime_capabilities(include_pro=include_pro, tier=tier)
        if isinstance(capabilities, dict):
            # Backward compatibility: older runtimes report requested_tier.
            if "fallback_tier" not in capabilities and "requested_tier" in capabilities:
                capabilities = dict(capabilities)
                capabilities["fallback_tier"] = capabilities.get("requested_tier")
                capabilities.pop("requested_tier", None)
        payload["capabilities"] = capabilities
    except RuntimeBootstrapError as exc:
        payload["status"] = "bootstrap_error"
        payload["error"] = str(exc)
    except Exception as exc:  # pragma: no cover
        payload["status"] = "error"
        payload["error"] = str(exc)

    return payload


def diagnostics_text(payload: dict[str, Any]) -> str:
    """Render a user-facing diagnostics report string."""
    status = payload.get("status", "unknown")
    lines = [
        "Whitebox Workflows Runtime Diagnostics",
        "",
        f"status: {status}",
        f"include_pro: {payload.get('include_pro')}",
        f"fallback_tier: {payload.get('fallback_tier')}",
    ]

    error = payload.get("error")
    if error:
        lines.extend(["", f"error: {error}"])

    capabilities = payload.get("capabilities")
    if isinstance(capabilities, dict):
        lines.extend(["", "capabilities:"])
        for key in sorted(capabilities.keys()):
            lines.append(f"  {key}: {capabilities[key]}")

    lines.extend(["", "json:", json.dumps(payload, indent=2, sort_keys=True)])
    return "\n".join(lines)
