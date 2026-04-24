# Tool Execution in QGIS

WbW-QGIS executes tools through the QGIS Processing framework.

## Typical Execution Path

1. Select a Whitebox algorithm in Processing Toolbox.
2. Fill parameters in the algorithm dialog.
3. Execute and monitor progress/messages.
4. Load or inspect output artifacts.

## Recommended Execution Practices

- Use explicit output paths for reproducibility.
- Start with small representative datasets before full runs.
- Validate intermediate outputs for CRS, schema, and metadata.
- Keep task logs for long workflows and batch operations.

## Output Handling

Whitebox tools may produce:
- raster outputs,
- vector outputs,
- lidar outputs,
- text/report sidecar artifacts.

Confirm output type and format before chaining into downstream steps.

## Progress and Messaging

Execution status and warnings should be treated as part of result validation.
If a tool completes with warnings, inspect outputs before continuing.
