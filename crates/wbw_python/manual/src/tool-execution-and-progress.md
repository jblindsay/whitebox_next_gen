# Tool Execution and Progress

This chapter documents execution styles and progress handling.

## Object-First Execution

```python
import whitebox_workflows as wb

wbe = wb.WbEnvironment()
dem = wbe.read_raster('dem.tif')

filled = wbe.hydrology.fill_depressions(dem)
accum = wbe.hydrology.d8_flow_accum(filled)
wbe.write_raster(accum, 'accum.tif')
```

## Path-First Execution

```python
import whitebox_workflows as wb

wbe = wb.WbEnvironment()

result = wbe.hydrology.fill_depressions(
	dem='dem.tif',
	output='filled.tif',
)
print(result)
```

## Basic Progress Callback

```python
import whitebox_workflows as wb

wbe = wb.WbEnvironment()

filled = wbe.hydrology.fill_depressions(
	dem='dem.tif',
	output='filled.tif',
	callback=wb.print_progress,
)
```

## Custom Progress Callback

```python
import json
import re

PERCENT_RE = re.compile(r'(-?\d+(?:\.\d+)?)\s*%')

def on_progress(event):
	payload = event
	if isinstance(event, str):
		try:
			payload = json.loads(event)
		except json.JSONDecodeError:
			payload = {'message': event}

	if isinstance(payload, dict):
		pct = payload.get('percent')
		msg = payload.get('message', '')
		if pct is None and msg:
			m = PERCENT_RE.search(str(msg))
			pct = float(m.group(1)) if m else None
		if pct is not None:
			if pct <= 1.0:
				pct *= 100.0
			print(f'[{int(max(0, min(100, pct))):3d}%] {msg}')

# Use: callback=on_progress
```

## Recommended Execution Pattern

1. Validate required tools.
2. Run tool chain memory-first.
3. Emit progress for long operations.
4. Persist only key outputs.
