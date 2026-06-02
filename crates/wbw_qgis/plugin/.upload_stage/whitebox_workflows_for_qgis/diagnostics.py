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

        csrs_support = capabilities.get("projection_csrs_preferred_operation_support")
        if isinstance(csrs_support, dict):
            zone_min = csrs_support.get("zone_min")
            zone_max = csrs_support.get("zone_max")
            lines.extend([
                "",
                "csrs_preferred_operation_support:",
                f"  zone_range: {zone_min}-{zone_max}",
            ])

            pairs = csrs_support.get("pairs")
            if isinstance(pairs, list):
                active_pairs = []
                pending_count = 0
                pending_corridors = []
                for pair in pairs:
                    if not isinstance(pair, dict):
                        continue
                    status = str(pair.get("status", "")).strip().lower()
                    src = str(pair.get("source_realization", "")).strip()
                    dst = str(pair.get("target_realization", "")).strip()
                    op_code = pair.get("preferred_operation_code")
                    if status == "active":
                        active_pairs.append((src, dst, op_code))
                    elif status == "pending":
                        pending_count += 1
                        if src == "v8" or dst == "v8":
                            pending_corridors.append((src, dst))

                if active_pairs:
                    lines.append("  active_pairs:")
                    for src, dst, op_code in active_pairs:
                        lines.append(f"    {src} -> {dst}: op={op_code}")
                lines.append(f"  pending_pair_count: {pending_count}")
                if pending_corridors:
                    lines.append("  pending_corridors_v8_family:")
                    for src, dst in sorted(set(pending_corridors)):
                        lines.append(f"    {src} -> {dst}")

    lines.extend(["", "json:", json.dumps(payload, indent=2, sort_keys=True)])
    return "\n".join(lines)
