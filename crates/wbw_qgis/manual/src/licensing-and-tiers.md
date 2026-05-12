# Licensing and Tiers

Whitebox NG is available in two licensing tiers with different capabilities and
licensing models.

## License Tiers

- **Open Tier** (free): Governed by MIT/Apache 2.0 dual licensing.
  All Open-tier tools are free and open-source with no entitlement or
  activation required. Use this tier for learning, research, and open
  development.

- **Pro Tier** (commercial): Proprietary software governed by EULA.
  Pro-tier tools provide advanced capabilities and require activation with a
  valid license key. Once activated, the license persists locally so you do not
  need to re-authenticate on each QGIS session.

## How QGIS Reflects Licensing

The Whitebox Workflows QGIS plugin is a frontend layer. Licensing authority and
rules are enforced in the backend runtime.

### Core Principle

The plugin reflects backend capabilities; it does not define licensing rules.
Runtime mode (open vs. pro) determines which tools are available and functional.

### Practical Behavior

- Open-tier tools are expected to run in all standard public QGIS environments.
- Pro-tier tools may be visible in the plugin but locked without an active Pro
  license.
- You can request a specific tier, but the effective tier depends on your
  entitlement state.

### Why This Matters

- One plugin surface adapts to both open and pro capability tiers.
- Tool discovery remains consistent across Python, R, and QGIS frontends.
- Licensing decisions and enforcement remain centralized in backend logic.

## Interactive License Management

The plugin provides convenient menu actions for license management without
requiring external tools or command lines.

### Activating a Pro License

1. In QGIS, navigate to **Plugins > Whitebox Workflows > Activate License**.
2. Enter your license information when prompted:
   - License key (required)
   - First name (required)
   - Last name (required)
   - Email (required)
   - Provider URL (optional; defaults to production)
3. Accept the EULA terms.
4. Click OK. The plugin will activate and persist your license locally.
5. The tool catalog automatically refreshes to show Pro-tier tools.

**Important:** License activation is tied to your machine. See
[Transferring a License](#transferring-a-license) to move to another machine.

### Checking License Status

Navigate to **Plugins > Whitebox Workflows > Plugin Settings** (or look for
diagnostics output) to see:
- License validity (active or expired)
- Effective tier (open or pro)
- License expiration time

### Transferring a License

If you need to use your Pro license on a different machine, you must first
deactivate it on the current machine and then activate on the destination.

1. On the **current machine**, navigate to **Plugins > Whitebox Workflows >
   Transfer License**. This generates a portable activation payload and clears
   your local license state.
2. Share the activation payload with the destination machine (or keep it for
   your own use on the other machine).
3. On the **destination machine**, navigate to **Plugins > Whitebox Workflows >
   Activate License** and enter your license key using the same process as
   above. The destination will obtain its own local license state.

### Deactivating a License

If you no longer plan to use Pro tools on this machine, navigate to
**Plugins > Whitebox Workflows > Deactivate License**. This clears your local
license state. Future sessions will fall back to Open-tier tools only.

## Local License State

Once activated, your license information is stored locally at
`~/.whitebox/wbw_ng_license_state.json` (or override via the
`WBW_LICENSE_STATE_PATH` environment variable). On each QGIS startup:
- If valid local state exists, it is automatically loaded.
- If local state is expired or missing, the plugin falls back to Open-tier mode.
- You do not need to re-authenticate in QGIS on every session.

## Expected Local-Dev Outcome

For most source-based setups, assume open-tier behavior unless your runtime
environment is explicitly configured for Pro-enabled integration testing or you
have an active Pro license activated on your machine.
