from __future__ import annotations

import json
from pathlib import Path

import whitebox_workflows as wb


DATA_ROOT = Path("/Users/johnlindsay/Documents/data")
OUTPUT_ROOT = Path(__file__).resolve().parent / "output" / "current_api_data_handling_demo"


def first_existing(paths: list[Path]) -> Path:
    for path in paths:
        if path.exists():
            return path
    raise FileNotFoundError(f"None of the candidate paths exist: {paths}")


def print_header(title: str) -> None:
    print(f"\n{'=' * 80}\n{title}\n{'=' * 80}")


def print_kv(label: str, value: object) -> None:
    print(f"{label}: {value}")


def raster_metadata_to_dict(meta: wb.RasterConfigs) -> dict[str, object]:
    return {
        "rows": meta.rows,
        "columns": meta.columns,
        "nodata": meta.nodata,
        "north": meta.north,
        "south": meta.south,
        "east": meta.east,
        "west": meta.west,
        "resolution_x": meta.resolution_x,
        "resolution_y": meta.resolution_y,
        "minimum": meta.minimum,
        "maximum": meta.maximum,
        "epsg_code": meta.epsg_code,
    }


def demonstrate_raster(wbe: wb.WbEnvironment, raster_path: Path, vector_path: Path) -> None:
    print_header("Raster: read, inspect metadata, create new rasters, write output")
    raster = wbe.read_raster(str(raster_path))
    metadata = raster.metadata()

    print_kv("Input raster", raster.file_path)
    print_kv("Short name", raster.get_short_filename())
    print_kv("Band count", raster.band_count)
    print_kv("CRS EPSG", raster.crs_epsg())
    print_kv("CRS WKT available", raster.crs_wkt() is not None)
    print("Raster.metadata() snapshot:")
    print(json.dumps(raster_metadata_to_dict(metadata), indent=2))

    center_row = metadata.rows // 2
    center_col = metadata.columns // 2
    print_kv("Center cell value", raster.get_value(center_row, center_col))
    print_kv("Number of cells", raster.num_cells())
    print_kv("Number of valid cells", raster.num_valid_cells())

    from_raster = wbe.new_raster_from_base_raster(
        base=raster,
        out_val=0.0,
        data_type="float",
        output_path=str(OUTPUT_ROOT / "raster_from_base_raster.tif"),
    )
    print_kv("Created raster from base raster", from_raster.file_path)

    from_vector = wbe.new_raster_from_base_vector(
        base=wbe.read_vector(str(vector_path)),
        cell_size=100.0,
        out_val=-9999.0,
        data_type="float",
        output_path=str(OUTPUT_ROOT / "raster_from_base_vector.tif"),
    )
    print_kv("Created raster from base vector", from_vector.file_path)

    raster_copy_path = OUTPUT_ROOT / "raster_copy_written.tif"
    wbe.write_raster(from_raster, str(raster_copy_path), compress=False)
    print_kv("Copied raster with write_raster", raster_copy_path)


def demonstrate_vector(wbe: wb.WbEnvironment, vector_path: Path) -> None:
    print_header("Vector: read, inspect metadata and attributes")
    vector = wbe.read_vector(str(vector_path))
    metadata = vector.metadata()

    print_kv("Input vector", vector.file_path)
    print_kv("Short name", vector.get_short_filename())
    print_kv("Feature count", metadata.feature_count)
    print_kv("CRS EPSG", metadata.crs_epsg)
    print_kv("CRS WKT available", metadata.crs_wkt is not None)
    print_kv("Attribute fields", vector.schema()[:10])

    if vector.feature_count() > 0:
        print("First feature attributes:")
        print(json.dumps(vector.attributes(0), indent=2, default=str))

    suffix = vector_path.suffix.lower()
    vector_copy = vector.deep_copy(str(OUTPUT_ROOT / f"vector_copy{suffix}"))
    print_kv("Copied vector with deep_copy", vector_copy.file_path)
    written_path = OUTPUT_ROOT / f"vector_written{suffix}"
    wbe.write_vector(vector_copy, str(written_path))
    print_kv("Copied vector with write_vector", written_path)


def demonstrate_lidar(wbe: wb.WbEnvironment, lidar_path: Path) -> None:
    print_header("Lidar: read, inspect metadata, copy, write output")
    lidar = wbe.read_lidar(str(lidar_path))
    metadata = lidar.metadata()

    print_kv("Input lidar", lidar.file_path)
    print_kv("Short name", lidar.get_short_filename())
    print_kv("File size (bytes)", metadata.file_size_bytes)
    print_kv("CRS EPSG", metadata.crs_epsg)
    print_kv("CRS WKT available", metadata.crs_wkt is not None)

    lidar_copy = lidar.deep_copy(str(OUTPUT_ROOT / f"lidar_copy{lidar_path.suffix.lower()}"))
    print_kv("Copied lidar with deep_copy", lidar_copy.file_path)

    written_path = OUTPUT_ROOT / f"lidar_written{lidar_path.suffix.lower()}"
    wbe.write_lidar(lidar_copy, str(written_path))
    print_kv("Copied lidar with write_lidar", written_path)


def main() -> None:
    OUTPUT_ROOT.mkdir(parents=True, exist_ok=True)

    raster_path = first_existing(
        [
            DATA_ROOT / "Ottawa DEM" / "DEM_8m.tif",
            DATA_ROOT / "vicksburg_dtm_ref_1sec_area.tif",
            DATA_ROOT / "Abbey_yield.tif",
        ]
    )
    vector_path = first_existing(
        [
            DATA_ROOT / "Ottawa DEM" / "Ottawa_soils_data.shp",
            DATA_ROOT / "Ponui island NZ" / "catchment1.shp",
            DATA_ROOT / "Edmonton_lidar" / "tmp1.shp",
        ]
    )
    lidar_path = first_existing(
        [
            DATA_ROOT / "Ponui island NZ" / "ponui" / "tmp1.las",
            DATA_ROOT / "Ponui island NZ" / "ponui" / "ponui_row6_col4.laz",
        ]
    )

    print_header("Current API Notes")
    print("Raster, Vector, and Lidar now each expose metadata() in the harmonized API direction.")
    print("Raster.configs() remains available as a compatibility path.")
    print("Generic creation helpers currently exist for rasters, not for vectors or lidars.")
    print(f"Output directory: {OUTPUT_ROOT}")

    wbe = wb.WbEnvironment()
    demonstrate_raster(wbe, raster_path, vector_path)
    demonstrate_vector(wbe, vector_path)
    demonstrate_lidar(wbe, lidar_path)


if __name__ == "__main__":
    main()