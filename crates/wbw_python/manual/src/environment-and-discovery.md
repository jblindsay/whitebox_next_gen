# Environment and Discovery

This chapter covers environment lifecycle and tool discovery patterns.

## Create and Configure Environment

```python
import whitebox_workflows as wb

wbe = wb.WbEnvironment()
wbe.working_directory = '/path/to/data'
wbe.verbose = True

print(wbe.version())
print(wbe.license_type())
```

## Namespace Discovery

```python
import whitebox_workflows as wb

wbe = wb.WbEnvironment()

print('categories:', wbe.categories())
print('domains:', wbe.domain_namespaces())
print('terrain tools sample:', wbe.terrain.list_tools()[:15])
```

## Search and Describe Tools

```python
import whitebox_workflows as wb

wbe = wb.WbEnvironment()

matches = wbe.search_tools('slope')
for m in matches[:5]:
	print(m.get('tool_id', m))

desc = wbe.describe_tool('slope')
print(desc)
```

## Validate Tool Availability

```python
import whitebox_workflows as wb

wbe = wb.WbEnvironment()
available = set(wbe.list_tools())

required = {'fill_depressions', 'd8_flow_accum'}
missing = sorted(required - available)
if missing:
	raise RuntimeError(f'Missing required tools: {missing}')
```

## Category Access Patterns

Prefer direct properties when available:
- `wbe.hydrology`
- `wbe.terrain`
- `wbe.raster`
- `wbe.vector`
- `wbe.lidar`
- `wbe.remote_sensing`

Use generic accessors for dynamic workflows:
- `wbe.category(name)`
- `wbe.domain(name)`
