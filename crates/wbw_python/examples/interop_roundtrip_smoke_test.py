from __future__ import annotations

from pathlib import Path
import tempfile
import json

import whitebox_workflows as wb


class SmokeResult:
    def __init__(self) -> None:
        self.passed: list[str] = []
        self.skipped: list[str] = []

    def pass_case(self, name: str) -> None:
        self.passed.append(name)

    def skip_case(self, name: str, reason: str) -> None:
        self.skipped.append(f"{name}: {reason}")


def _maybe_import_numpy():
    try:
        import numpy as np  # type: ignore

        return np
    except ImportError:
        return None


def _maybe_import_rasterio():
    try:
        import rasterio  # type: ignore
        from rasterio.transform import from_origin  # type: ignore

        return rasterio, from_origin
    except ImportError:
        return None


def _maybe_import_geopandas_shapely():
    try:
        import geopandas as gpd  # type: ignore
        from shapely.geometry import Point  # type: ignore

        return gpd, Point
    except ImportError:
        return None


def _maybe_import_pyproj():
    try:
        from pyproj import CRS  # type: ignore

        return CRS
    except ImportError:
        return None


def baseline_runtime_smoke(result: SmokeResult) -> None:
    session = wb.RuntimeSession()
    tools = json.loads(session.list_tools_json())
    if not isinstance(tools, list) or not tools:
        raise AssertionError("baseline runtime smoke found no registered tools")
    result.pass_case("baseline_runtime")


def raster_roundtrip_smoke(wbe: wb.WbEnvironment, tmp_dir: Path, result: SmokeResult) -> wb.Raster | None:
    np = _maybe_import_numpy()
    rasterio_bundle = _maybe_import_rasterio()
    if np is None:
        result.skip_case("raster_roundtrip", "numpy not installed")
        return None
    if rasterio_bundle is None:
        result.skip_case("raster_roundtrip", "rasterio not installed")
        return None

    rasterio, from_origin = rasterio_bundle
    seed = tmp_dir / "seed.tif"
    roundtrip = tmp_dir / "roundtrip.tif"
    exported = tmp_dir / "exported.tif"

    arr = np.array([[1.0, 2.0], [3.0, 4.0]], dtype=np.float32)
    with rasterio.open(
        seed,
        "w",
        driver="GTiff",
        height=arr.shape[0],
        width=arr.shape[1],
        count=1,
        dtype="float32",
        crs="EPSG:4326",
        transform=from_origin(0.0, 2.0, 1.0, 1.0),
        nodata=-9999.0,
    ) as dst:
        dst.write(arr, 1)

    ras = wbe.read_raster(str(seed))
    as_np = ras.to_numpy(dtype="float64")
    out_arr = as_np + 2.0

    out_raster = wb.Raster.from_numpy(out_arr, ras, output_path=str(roundtrip))
    reloaded = wbe.read_raster(out_raster.file_path)
    back = reloaded.to_numpy(dtype="float64")
    if not np.allclose(back, out_arr):
        raise AssertionError("raster numpy roundtrip values changed unexpectedly")

    wbe.write_raster(reloaded, str(exported))
    with rasterio.open(exported) as src:
        if src.read(1).shape != arr.shape:
            raise AssertionError("exported raster shape mismatch")
        if src.crs is None:
            raise AssertionError("exported raster CRS missing")

    result.pass_case("raster_roundtrip")
    return reloaded


def vector_roundtrip_smoke(wbe: wb.WbEnvironment, tmp_dir: Path, result: SmokeResult) -> None:
    bundle = _maybe_import_geopandas_shapely()
    if bundle is None:
        result.skip_case("vector_roundtrip", "geopandas/shapely not installed")
        return

    gpd, Point = bundle
    seed = tmp_dir / "seed.gpkg"
    out_path = tmp_dir / "out.gpkg"

    gdf = gpd.GeoDataFrame(
        {"name": ["a", "b"], "value": [1, 2]},
        geometry=[Point(0.0, 0.0), Point(1.0, 1.0)],
        crs="EPSG:4326",
    )
    gdf.to_file(seed, driver="GPKG")

    vec = wbe.read_vector(str(seed))
    if vec.feature_count() != 2:
        raise AssertionError("vector feature_count mismatch")

    attrs0 = vec.attributes(0)
    if "name" not in attrs0:
        raise AssertionError("vector attribute missing expected field 'name'")

    wbe.write_vector(vec, str(out_path))
    gdf2 = gpd.read_file(out_path)
    if len(gdf2) != 2:
        raise AssertionError("vector roundtrip row count mismatch")
    if "name" not in gdf2.columns:
        raise AssertionError("vector roundtrip missing 'name' column")

    result.pass_case("vector_roundtrip")


def pyproj_crs_smoke(reference_raster: wb.Raster | None, result: SmokeResult) -> None:
    CRS = _maybe_import_pyproj()
    if CRS is None:
        result.skip_case("pyproj_crs", "pyproj not installed")
        return

    if reference_raster is None:
        result.skip_case("pyproj_crs", "no raster CRS source available")
        return

    epsg = reference_raster.metadata().epsg_code
    if epsg is None or epsg == 0:
        result.skip_case("pyproj_crs", "raster metadata did not provide EPSG")
        return

    crs = CRS.from_epsg(epsg)
    if crs.to_epsg() != epsg:
        raise AssertionError("pyproj EPSG roundtrip mismatch")

    result.pass_case("pyproj_crs")


def main() -> None:
    wbe = wb.WbEnvironment()
    result = SmokeResult()

    baseline_runtime_smoke(result)

    with tempfile.TemporaryDirectory(prefix="wbw_py_interop_smoke_") as td:
        tmp_dir = Path(td)
        reference_raster = raster_roundtrip_smoke(wbe, tmp_dir, result)
        vector_roundtrip_smoke(wbe, tmp_dir, result)
        pyproj_crs_smoke(reference_raster, result)

    print("interop smoke test summary")
    print("  passed:")
    for name in result.passed:
        print(f"    - {name}")

    print("  skipped:")
    if result.skipped:
        for item in result.skipped:
            print(f"    - {item}")
    else:
        print("    - none")

    optional_passed = [name for name in result.passed if name != "baseline_runtime"]
    if not optional_passed:
        print(
            "No optional interoperability bridge executed. Install optional dependencies"
            " (numpy/rasterio/geopandas/shapely/pyproj) for deeper coverage."
        )

    print("interop roundtrip smoke test passed")


if __name__ == "__main__":
    main()
