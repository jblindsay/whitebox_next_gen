"""Floating-license online bootstrap example for Whitebox Workflows Python.

This example is intended for machines that do not yet have a local
license-state file. The runtime will attempt floating-ID activation via
provider bootstrap when include_pro=True.
"""

import json
import os

import whitebox_workflows


def main() -> None:
    # Required for provider bootstrap.
    os.environ["WBW_LICENSE_PROVIDER_URL"] = "https://your-provider.example.com"
    os.environ["WBW_FLOATING_LICENSE_ID"] = "FLOAT-ABC-123"

    # Optional policy and tuning.
    os.environ.setdefault("WBW_LICENSE_POLICY", "fail_open")
    os.environ.setdefault("WBW_LICENSE_LEASE_SECONDS", "3600")
    # Optional explicit state path (otherwise platform default location is used).
    # os.environ["WBW_LICENSE_STATE_PATH"] = "/tmp/wbw_license_state.json"

    # Optional machine/customer hints used during floating activation.
    # os.environ["WBW_MACHINE_ID"] = "my-workstation-01"
    # os.environ["WBW_CUSTOMER_ID"] = "cust_123"

    session = whitebox_workflows.RuntimeSession(include_pro=True, tier="open")

    tools = json.loads(session.list_tools_json())
    tool_ids = {tool["id"] for tool in tools}
    print(f"Num tools visible: {len(tools)}")
    print(f"Pro tool visible (raster_power): {'raster_power' in tool_ids}")


if __name__ == "__main__":
    main()
