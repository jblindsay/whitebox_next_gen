"""Offline licensing examples for Whitebox Workflows Python.

Demonstrates:
1) OSS-only fallback mode (no provider URL)
2) Explicit signed-entitlement mode (no provider call)
"""

import json
import os

import whitebox_workflows


def offline_oss_fallback() -> None:
    # No provider URL means no network bootstrap attempt.
    os.environ.pop("WBW_LICENSE_PROVIDER_URL", None)
    os.environ.pop("WBW_FLOATING_LICENSE_ID", None)

    # In offline/no-provider mode, this stays in tier-only behavior.
    session = whitebox_workflows.RuntimeSession(include_pro=False, tier="open")
    tools = json.loads(session.list_tools_json())
    print(f"Offline OSS tool count: {len(tools)}")


def offline_with_signed_entitlement() -> None:
    # If you already have a signed entitlement JSON envelope and public key,
    # you can run offline without provider bootstrap.
    entitlement_path = "./signed_entitlement.json"
    public_key_kid = "k1"
    public_key_b64url = "REPLACE_WITH_PROVIDER_PUBLIC_KEY"

    if not os.path.exists(entitlement_path):
        print(
            "Skipping signed entitlement example; provide ./signed_entitlement.json first."
        )
        return

    with open(entitlement_path, "r", encoding="utf-8") as f:
        signed_entitlement_json = f.read()

    session = whitebox_workflows.RuntimeSession.from_signed_entitlement_json(
        signed_entitlement_json=signed_entitlement_json,
        public_key_kid=public_key_kid,
        public_key_b64url=public_key_b64url,
        include_pro=True,
        fallback_tier="open",
    )

    tools = json.loads(session.list_tools_json())
    print(f"Offline entitlement-based tool count: {len(tools)}")


def main() -> None:
    offline_oss_fallback()
    offline_with_signed_entitlement()


if __name__ == "__main__":
    main()
