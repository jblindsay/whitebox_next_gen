from __future__ import annotations

from pathlib import Path

import whitebox_workflows as wb


DATA_ROOT = Path('/Users/johnlindsay/Documents/data')
OUTPUT_DIR = Path(__file__).resolve().parent / 'output' / 'vector_multifile_write_demo'


def main() -> None:
    wbe = wb.WbEnvironment()
    OUTPUT_DIR.mkdir(parents=True, exist_ok=True)

    source = wbe.read_vector(str(DATA_ROOT / 'Ottawa DEM' / 'Ottawa_soils_data.shp'))

    shp_out = OUTPUT_DIR / 'out_shapefile.shp'
    wbe.write_vector(source, str(shp_out))
    print('Shapefile outputs:')
    print('  .shp:', shp_out.exists())
    print('  .dbf:', shp_out.with_suffix('.dbf').exists())
    print('  .shx:', shp_out.with_suffix('.shx').exists())

    mif_out = OUTPUT_DIR / 'out_mapinfo.mif'
    wbe.write_vector(source, str(mif_out))
    print('MapInfo outputs:')
    print('  .mif:', mif_out.exists())
    print('  .mid:', mif_out.with_suffix('.mid').exists())


if __name__ == '__main__':
    main()
