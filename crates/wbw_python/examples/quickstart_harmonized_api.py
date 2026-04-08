from __future__ import annotations

from pathlib import Path

import whitebox_workflows as wb


DATA_ROOT = Path('/Users/johnlindsay/Documents/data')


def first_existing(paths: list[Path]) -> Path:
    for path in paths:
        if path.exists():
            return path
    raise FileNotFoundError(f'None of these paths exist: {paths}')


def main() -> None:
    wbe = wb.WbEnvironment()

    raster_path = first_existing([
        DATA_ROOT / 'Ottawa DEM' / 'DEM_8m.tif',
        DATA_ROOT / 'Abbey_yield.tif',
    ])
    vector_path = first_existing([
        DATA_ROOT / 'Ottawa DEM' / 'Ottawa_soils_data.shp',
        DATA_ROOT / 'Ponui island NZ' / 'catchment1.shp',
    ])
    lidar_path = first_existing([
        DATA_ROOT / 'Ponui island NZ' / 'ponui' / 'tmp1.las',
        DATA_ROOT / 'Ponui island NZ' / 'ponui' / 'ponui_row6_col4.laz',
    ])

    raster = wbe.read_raster(str(raster_path))
    rmeta = raster.metadata()
    print('Raster metadata:', rmeta.rows, rmeta.columns, rmeta.nodata)

    vector = wbe.read_vector(str(vector_path))
    vmeta = vector.metadata()
    print('Vector metadata:', vmeta.feature_count, len(vmeta.attribute_field_names))

    lidar = wbe.read_lidar(str(lidar_path))
    lmeta = lidar.metadata()
    print('Lidar metadata:', lmeta.file_size_bytes, lmeta.crs_epsg)


if __name__ == '__main__':
    main()
