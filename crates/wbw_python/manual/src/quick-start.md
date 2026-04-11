# Quick Start

```python
import whitebox_workflows as wb

wbe = wb.WbEnvironment()
dem = wbe.read_raster("dem.tif")
filled = wbe.hydrology.fill_depressions(dem)
wbe.write_raster(filled, "dem_filled.tif")
```

Planned expansion:
- Session/environment lifecycle.
- Discovery patterns.
- Memory-first chaining versus explicit output writes.
