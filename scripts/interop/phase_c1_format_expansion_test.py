#!/usr/bin/env python3
"""Phase C.1 format expansion runner (R09-R11, V05-V07)."""

from __future__ import annotations

import json
import shutil
import subprocess
import sys
import tempfile
from datetime import datetime
from pathlib import Path

ROOT = Path("/Users/johnlindsay/Documents/programming/Rust/whitebox_next_gen")
RESULTS_ROOT = ROOT / "artifacts/interop/results"
RESULTS_FILE = RESULTS_ROOT / "phase_c1_format_expansion_results.json"


def run_cmd(cmd: str, timeout: int = 120):
    try:
        proc = subprocess.run(
            cmd,
            shell=True,
            capture_output=True,
            text=True,
            timeout=timeout,
        )
        return proc.returncode, proc.stdout.strip(), proc.stderr.strip()
    except subprocess.TimeoutExpired:
        return 1, "", "Timeout"
    except Exception as exc:
        return 1, "", str(exc)


def get_env():
    try:
        import whitebox_workflows as wbw

        return wbw.WbEnvironment(), None
    except Exception as exc:
        return None, f"whitebox_workflows import failed: {exc}"


def create_vrt_raster(path: Path, data_type="Float32", value="7.25", nodata="-9999"):
    vrt = f'''<VRTDataset rasterXSize="20" rasterYSize="20">
  <SRS>EPSG:4326</SRS>
  <GeoTransform>-80, 0.01, 0, 44, 0, -0.01</GeoTransform>
  <VRTRasterBand dataType="{data_type}" band="1">
    <NoDataValue>{nodata}</NoDataValue>
    <ConstantBand val="{value}"/>
  </VRTRasterBand>
</VRTDataset>'''
    with tempfile.NamedTemporaryFile(mode="w", suffix=".vrt", delete=False) as f:
        f.write(vrt)
        vrt_path = Path(f.name)
    rc, _, err = run_cmd(f"gdal_translate -of GTiff '{vrt_path}' '{path}'")
    vrt_path.unlink(missing_ok=True)
    if rc != 0:
        return False, f"gdal_translate failed: {err}"
    return True, ""


def raster_roundtrip(source: Path, roundtrip: Path):
    env, err = get_env()
    if env is None:
        return "FAIL", err
    try:
        src = env.read_raster(str(source))
        env.write_raster(src, str(roundtrip))
        out = env.read_raster(str(roundtrip))
        if src.num_cells() != out.num_cells() or src.band_count != out.band_count:
            return "FAIL", "Mismatch in cell or band count"
        return "PASS", f"cells={src.num_cells()}, bands={src.band_count}"
    except Exception as exc:
        msg = str(exc)
        low = msg.lower()
        if "not implemented" in low or "unsupported" in low or "unknown format" in low:
            return "NOT_STARTED", msg
        return "FAIL", msg


def write_sample_geojson(path: Path):
    fc = {
        "type": "FeatureCollection",
        "features": [
            {
                "type": "Feature",
                "properties": {"id": 1, "name": "alpha"},
                "geometry": {"type": "Point", "coordinates": [-79.5, 43.5]},
            },
            {
                "type": "Feature",
                "properties": {"id": 2, "name": "beta"},
                "geometry": {"type": "Point", "coordinates": [-79.6, 43.6]},
            },
        ],
    }
    path.write_text(json.dumps(fc), encoding="utf-8")


def vector_roundtrip(source: Path, roundtrip: Path):
    env, err = get_env()
    if env is None:
        return "FAIL", err
    try:
        src = env.read_vector(str(source))
        env.write_vector(src, str(roundtrip))
        out = env.read_vector(str(roundtrip))
        src_count = src.feature_count()
        out_count = out.feature_count()
        if src_count != out_count:
            return "FAIL", f"Feature count mismatch: {src_count} vs {out_count}"
        return "PASS", f"features={src_count}"
    except Exception as exc:
        msg = str(exc)
        low = msg.lower()
        if "not implemented" in low or "unsupported" in low or "unknown format" in low:
            return "NOT_STARTED", msg
        return "FAIL", msg


def test_r09():
    """R09: GeoPackage raster container."""
    d = RESULTS_ROOT / "raster/R09"
    d.mkdir(parents=True, exist_ok=True)
    base = d / "_tmp_base.tif"
    source = d / "source_gdal.gpkg"
    out = d / "roundtrip_wbraster.gpkg"
    ok, msg = create_vrt_raster(base)
    if not ok:
        return "FAIL", msg
    rc, _, err = run_cmd(f"gdal_translate -of GPKG '{base}' '{source}'")
    base.unlink(missing_ok=True)
    if rc != 0:
        return "FAIL", f"GeoPackage raster creation failed: {err}"
    return raster_roundtrip(source, out)


def test_r10():
    """R10: Esri ASCII Grid."""
    d = RESULTS_ROOT / "raster/R10"
    d.mkdir(parents=True, exist_ok=True)
    base = d / "_tmp_base.tif"
    source = d / "source_gdal.asc"
    out = d / "roundtrip_wbraster.asc"
    ok, msg = create_vrt_raster(base)
    if not ok:
        return "FAIL", msg
    rc, _, err = run_cmd(f"gdal_translate -of AAIGrid '{base}' '{source}'")
    base.unlink(missing_ok=True)
    if rc != 0:
        return "FAIL", f"AAIGrid creation failed: {err}"
    return raster_roundtrip(source, out)


def test_r11():
    """R11: JPEG2000 (lossy policy case)."""
    d = RESULTS_ROOT / "raster/R11"
    d.mkdir(parents=True, exist_ok=True)
    base = d / "_tmp_base.tif"
    source = d / "source_gdal.jp2"
    out = d / "roundtrip_wbraster.jp2"
    ok, msg = create_vrt_raster(base, data_type="Byte", value="120", nodata="0")
    if not ok:
        return "FAIL", msg
    rc, _, err = run_cmd(
        f"gdal_translate -of JP2OpenJPEG -co QUALITY=35 '{base}' '{source}'"
    )
    base.unlink(missing_ok=True)
    if rc != 0:
        return "FAIL", f"JPEG2000 creation failed: {err}"
    status, note = raster_roundtrip(source, out)
    if status == "PASS":
        note = f"{note}; lossy-policy=tolerant"
    return status, note


def test_v05():
    """V05: KML/KMZ."""
    d = RESULTS_ROOT / "vector/V05"
    d.mkdir(parents=True, exist_ok=True)
    src_geojson = d / "source_input.geojson"
    source = d / "source_gdal.kml"
    out = d / "roundtrip_wbvector.kml"
    write_sample_geojson(src_geojson)
    rc, _, err = run_cmd(f"ogr2ogr -f LIBKML '{source}' '{src_geojson}'")
    if rc != 0:
        rc, _, err = run_cmd(f"ogr2ogr -f KML '{source}' '{src_geojson}'")
    if rc != 0:
        return "FAIL", f"KML creation failed: {err}"
    return vector_roundtrip(source, out)


def test_v06():
    """V06: GPX."""
    d = RESULTS_ROOT / "vector/V06"
    d.mkdir(parents=True, exist_ok=True)
    src_geojson = d / "source_input.geojson"
    source = d / "source_gdal.gpx"
    out = d / "roundtrip_wbvector.gpx"
    write_sample_geojson(src_geojson)
    rc, _, err = run_cmd(f"ogr2ogr -f GPX '{source}' '{src_geojson}'")
    if rc != 0:
        return "FAIL", f"GPX creation failed: {err}"
    return vector_roundtrip(source, out)


def test_v07():
    """V07: GML."""
    d = RESULTS_ROOT / "vector/V07"
    d.mkdir(parents=True, exist_ok=True)
    src_geojson = d / "source_input.geojson"
    source = d / "source_gdal.gml"
    out = d / "roundtrip_wbvector.gml"
    write_sample_geojson(src_geojson)
    rc, _, err = run_cmd(f"ogr2ogr -f GML '{source}' '{src_geojson}'")
    if rc != 0:
        return "FAIL", f"GML creation failed: {err}"
    return vector_roundtrip(source, out)


def main() -> int:
    print("=" * 70)
    print("Phase C.1 Format Expansion Runner")
    print(f"Timestamp: {datetime.now().isoformat()}")
    print("=" * 70)
    print()

    cases = {
        "R09": ("GeoPackage Raster", test_r09),
        "R10": ("AAIGrid ASCII", test_r10),
        "R11": ("JPEG2000 lossy", test_r11),
        "V05": ("KML/KMZ", test_v05),
        "V06": ("GPX", test_v06),
        "V07": ("GML", test_v07),
    }

    results = {}
    for cid in sorted(cases):
        desc, fn = cases[cid]
        print(f"{cid}: {desc}...", end=" ", flush=True)
        status, note = fn()
        results[cid] = {"status": status, "description": desc, "note": note}
        if status == "PASS":
            print("✓ PASS")
        elif status == "NOT_STARTED":
            print("⊘ NOT_STARTED")
        else:
            print("✗ FAIL")

    passed = sum(1 for r in results.values() if r["status"] == "PASS")
    failed = sum(1 for r in results.values() if r["status"] == "FAIL")
    skipped = sum(1 for r in results.values() if r["status"] == "NOT_STARTED")

    print()
    print("=" * 70)
    print("Summary")
    print("=" * 70)
    print(f"Total:       {len(results)} cases")
    print(f"Passed:      {passed}")
    print(f"Failed:      {failed}")
    print(f"Not Started: {skipped}")

    RESULTS_FILE.parent.mkdir(parents=True, exist_ok=True)
    RESULTS_FILE.write_text(json.dumps(results, indent=2) + "\n", encoding="utf-8")
    print(f"Results saved: {RESULTS_FILE}")

    return 0 if failed == 0 else 1


if __name__ == "__main__":
    sys.exit(main())
