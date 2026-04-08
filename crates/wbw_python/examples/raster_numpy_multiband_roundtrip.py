from __future__ import annotations

from pathlib import Path

import whitebox_workflows as wb


DATA_ROOT = Path('/Users/johnlindsay/Documents/data')
OUTPUT_DIR = Path(__file__).resolve().parent / 'output' / 'raster_numpy_multiband_roundtrip'


def first_existing(paths: list[Path]) -> Path:
    for path in paths:
        if path.exists():
            return path
    raise FileNotFoundError(f'None of these paths exist: {paths}')


def main() -> None:
    wbe = wb.WbEnvironment()
    OUTPUT_DIR.mkdir(parents=True, exist_ok=True)

    raster_path = first_existing([
        DATA_ROOT / 'Ottawa DEM' / 'DEM_8m.tif',
        DATA_ROOT / 'Abbey_yield.tif',
    ])

    raster = wbe.read_raster(str(raster_path))

    try:
        import numpy as np
    except ImportError as exc:
        print(exc)
        print('Install NumPy in your environment to run this example.')
        return

    # Request all bands in bands-first layout: (bands, rows, cols).
    arr_brc = raster.to_numpy(dtype='float64', all_bands=True)
    print('bands-first shape:', arr_brc.shape)

    out_bands_first = wb.Raster.from_numpy(
        arr_brc,
        raster,
        output_path=str(OUTPUT_DIR / 'from_bands_first.tif'),
    )
    print('Wrote:', out_bands_first.file_path)

    # Convert to rows-cols-bands and write again.
    arr_rcb = np.moveaxis(arr_brc, 0, -1)
    print('rows-cols-bands shape:', arr_rcb.shape)

    out_rows_cols_bands = wb.Raster.from_numpy(
        arr_rcb,
        raster,
        output_path=str(OUTPUT_DIR / 'from_rows_cols_bands.tif'),
    )
    print('Wrote:', out_rows_cols_bands.file_path)


if __name__ == '__main__':
    main()
