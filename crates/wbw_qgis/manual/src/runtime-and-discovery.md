# Runtime and Discovery

WbW-QGIS discovers tool availability at runtime using the active
whitebox_workflows environment.

## Discovery Flow

At a high level:
1. Import whitebox_workflows in the QGIS Python environment.
2. Create a runtime session.
3. Read runtime capability metadata.
4. Read tool catalog metadata.
5. Partition available vs locked tools.
6. Refresh Processing provider algorithms.

## When to Refresh

Refresh discovery when:
- plugin settings change,
- runtime tier/entitlement changes,
- whitebox_workflows is rebuilt or reinstalled,
- tool taxonomy updates are introduced.

## Common Discovery Symptoms

- Provider missing entirely: plugin import/runtime bootstrap failure.
- Provider appears but tools are absent: catalog read failure or stale cache.
- Tools show as locked unexpectedly: runtime capability/tier mismatch.

In all cases, validate environment alignment first.
