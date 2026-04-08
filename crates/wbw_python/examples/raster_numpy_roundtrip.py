from __future__ import annotations

from pathlib import Path

import whitebox_workflows as wb


DATA_ROOT = Path('/Users/johnlindsay/Documents/data')
OUTPUT_DIR = Path(__file__).resolve().parent / 'output' / 'raster_numpy_roundtrip'


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
        arr = raster.to_numpy(dtype='float64')
    except ImportError as exc:
        print(exc)
        print('Install NumPy in your environment to run this example.')
        return

    print('Array shape:', arr.shape)
    arr = arr + 1.0

    out_raster = wb.Raster.from_numpy(
        arr,
        raster,
        output_path=str(OUTPUT_DIR / 'raster_from_numpy.tif'),
    )
    print('Wrote:', out_raster.file_path)


if __name__ == '__main__':
    main()
